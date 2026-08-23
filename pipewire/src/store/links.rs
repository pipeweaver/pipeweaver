//! Lifecycle of the links we create. This is the fiddliest bit of the
//! store: a "link" is really a pair of per-channel pipewire links (left and
//! right) that both need to bind and get a pipewire id before we consider
//! the managed link ready, so most of this module is bookkeeping around
//! `pending_link_syncs` until that's true.

use super::{ManagedLink, ManagedLinkMap, PendingLinkSync, PortLocation, Store};
use crate::{Direction, LinkType, PipewireReceiver};
use anyhow::{Result, anyhow, bail};
use log::{debug, error, warn};
use oneshot::Sender;
use pipewire::core::CoreRc;
use pipewire::keys::{
    LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, NODE_PASSIVE,
    OBJECT_LINGER,
};
use pipewire::link::{Link, LinkChangeMask, LinkState};
use pipewire::properties::properties;
use pipewire::proxy::ProxyT;
use pipewire::registry::RegistryRc;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Weak;
use std::str::FromStr;
use strum::IntoEnumIterator;
use ulid::Ulid;

impl Store {
    // ----- MANAGED LINKS -----
    pub fn is_managed_link(&self, id: u32) -> Option<Ulid> {
        self.managed_links
            .iter()
            .find(|(_, node)| {
                PortLocation::iter().any(|port| {
                    node.links[port]
                        .as_ref()
                        .is_some_and(|link| link.pw_id == Some(id))
                })
            })
            .map(|(id, _)| id)
            .copied()
    }

    pub fn create_link(
        &mut self,
        core: &CoreRc,
        registry: &RegistryRc,
        source: LinkType,
        dest: LinkType,
        sender: Sender<()>,
        listener_store: Weak<RefCell<Store>>,
    ) -> Result<()> {
        // Fetch the details of the links that need creating
        let mut group = self.prepare_links(source, dest, sender, registry)?;

        // Create a Parent ID for this link set
        let parent_id = Ulid::generate();

        // Find the first portmap and begin creation
        for port in PortLocation::iter() {
            let entry = &mut group.links[port];

            if let Some(m) = entry.as_mut() {
                // Ok, we have a LinkStoreMap, trigger the first creation.
                create_port_link(core, parent_id, m, listener_store.clone())?;
                break;
            }
        }
        self.add_pending_link(parent_id, group);
        Ok(())
    }

    pub fn prepare_links(
        &mut self,
        source: LinkType,
        dest: LinkType,
        sender: Sender<()>,
        registry: &RegistryRc,
    ) -> Result<ManagedLink> {
        // First, check if a managed link already exists and remove it
        self.managed_link_remove(&source, &dest);
        let mut port_map: enum_map::EnumMap<PortLocation, Option<ManagedLinkMap>> =
            Default::default();

        // Collect all the port pairs we're going to link
        let mut port_pairs = Vec::new();
        for port in PortLocation::iter() {
            let (_, src_index) = self.get_port(&source, Direction::Out, port)?;
            let (_, tgt_index) = self.get_port(&dest, Direction::In, port)?;
            port_pairs.push((src_index, tgt_index));
        }

        // Find and destroy any unmanaged links between these exact ports
        let links_to_destroy: Vec<u32> = self
            .get_unmanaged_links()
            .iter()
            .filter_map(|(id, link)| {
                let should_remove = port_pairs.iter().any(|(out_port, in_port)| {
                    link.output_port == *out_port && link.input_port == *in_port
                });
                if should_remove { Some(*id) } else { None }
            })
            .collect();

        if !links_to_destroy.is_empty() {
            debug!(
                "Destroying {} orphaned unmanaged links in PipeWire: {:?}",
                links_to_destroy.len(),
                links_to_destroy
            );
            for link_id in links_to_destroy {
                registry.destroy_global(link_id);
                self.unmanaged_link_remove(link_id);
            }
        }

        // Now create the links
        for port in PortLocation::iter() {
            // Firstly, create an id for this list
            let link_id = Ulid::generate();

            // Next, obtain the source and destination port indexes
            let (src_id, src_index) = self.get_port(&source, Direction::Out, port)?;
            let (tgt_id, tgt_index) = self.get_port(&dest, Direction::In, port)?;

            // Create the LinkStore Mapping for this link
            let store = ManagedLinkMap {
                pw_id: None,
                internal_id: link_id,

                pending_seq_id: None,
                _link: None,
                _proxy_listener: None,
                _info_listener: None,

                source_port: (src_id, src_index),
                destination_port: (tgt_id, tgt_index),
            };

            port_map[port] = Some(store);
        }

        // Ok, we're done here, create the main store object
        let group = ManagedLink {
            source: source.clone(),
            destination: dest.clone(),
            links: port_map,
            ready_sender: Some(sender),
        };

        Ok(group)
    }

    pub fn remove_link(&mut self, source: LinkType, destination: LinkType) -> Result<()> {
        self.managed_link_remove(&source, &destination);
        Ok(())
    }

    pub(crate) fn get_port(
        &mut self,
        link: &LinkType,
        direction: Direction,
        location: PortLocation,
    ) -> Result<(u32, u32)> {
        // Ok, simple enough, pull out the relevant type, and get the port at location
        match link {
            LinkType::Node(id) => {
                let Some(node) = self.managed_node_get(*id) else {
                    bail!("Unable to Locate Node");
                };

                let id = node.pw_id.unwrap();
                let port = node.port_map[location].unwrap();

                Ok((id, port))
            }
            LinkType::Filter(id) => {
                let filter = self.managed_filter_get(*id).unwrap();

                let id = filter.pw_id.unwrap();
                let port = filter.port_map[direction][location];

                Ok((id, port))
            }
            LinkType::UnmanagedNode(id, port_map) => {
                let node = self
                    .unmanaged_device_node_get(*id)
                    .ok_or_else(|| anyhow!("Unmanaged Device Node not Found"))?;

                let ports = &node.ports[direction];

                // Check whether the caller has explicitly told us which port to use
                if let Some(link_ports) = port_map {
                    let find = match location {
                        PortLocation::Left => &link_ports.left,
                        PortLocation::Right => &link_ports.right,
                    };

                    for (index, port) in ports.iter() {
                        if port.channel == *find {
                            return Ok((*id, *index));
                        }
                    }

                    bail!("Unable to find channel");
                }

                // Check whether this is a mono device
                if ports.iter().count() == 1
                    && let Some(index) = ports.keys().next()
                {
                    return Ok((*id, *index));
                }

                // Iterate over the ports, try and find the location
                for (index, port) in ports.iter() {
                    if let Ok(port_location) = PortLocation::from_str(&port.channel)
                        && port_location == location
                    {
                        return Ok((*id, *index));
                    }
                }

                // If we get here, we didn't find anything, this shouldn't happen!
                bail!("Requested Unmanaged Node is Neither Stereo or Mono");
            }
        }
    }

    pub fn managed_link_add(&mut self, id: Ulid, group: ManagedLink) {
        self.managed_links.insert(id, group);
    }

    pub fn add_pending_link(&mut self, parent_id: Ulid, group: ManagedLink) {
        self.pending_link_syncs.push(PendingLinkSync {
            parent_id,
            group,
            bound_ids: HashMap::new(),
        });
    }

    pub fn get_next_pending_link(&mut self, seq: i32) -> Option<&mut ManagedLinkMap> {
        let idx = self.get_pending_link_index_by_seq(seq)?;

        // Scope to limit the mutable borrow
        let port = {
            let pending = &self.pending_link_syncs[idx];

            PortLocation::iter().find(|&port| {
                pending.group.links[port]
                    .as_ref()
                    .is_some_and(|link| link.pending_seq_id.is_none())
            })
        };

        if let Some(port) = port {
            return self.pending_link_syncs[idx].group.links[port].as_mut();
        }

        // No ports left, take ownership and finish up.
        let pending = self.pending_link_syncs.remove(idx);
        let mut group = pending.group;

        for port in PortLocation::iter() {
            if let Some(link) = &mut group.links[port]
                && let Some(pw_id) = pending.bound_ids.get(&link.internal_id)
            {
                link.pw_id = Some(*pw_id);
            }
        }

        debug!("Link Created {:?} to {:?}", group.source, group.destination);
        self.managed_link_add(pending.parent_id, group);
        self.managed_link_ready_check(pending.parent_id);

        None
    }

    fn get_pending_link_index_by_seq(&self, seq: i32) -> Option<usize> {
        self.pending_link_syncs.iter().position(|pending| {
            PortLocation::iter().any(|port| {
                pending.group.links[port]
                    .as_ref()
                    .is_some_and(|info| info.pending_seq_id == Some(seq))
            })
        })
    }

    pub fn get_pending_link_parent_id_by_seq(&self, seq: i32) -> Option<Ulid> {
        // We need to iterate over all the pending link syncs, and see if a port has our seq
        // id, if so return that parent.
        self.pending_link_syncs.iter().find_map(|pending| {
            PortLocation::iter().find_map(|port| {
                pending.group.links[port].as_ref().and_then(|info| {
                    (info.pending_seq_id == Some(seq)).then_some(pending.parent_id)
                })
            })
        })
    }

    pub fn set_pending_link_done(&mut self, parent_id: Ulid, link_id: Ulid, seq_id: i32) {
        for pending in &mut self.pending_link_syncs {
            if pending.parent_id != parent_id {
                continue;
            }
            for port in PortLocation::iter() {
                if let Some(info) = pending.group.links[port].as_mut()
                    && info.internal_id == link_id
                {
                    info.pending_seq_id = Some(seq_id);
                    return; // stop as soon as it's found
                }
            }
        }
    }

    pub fn managed_link_remove(&mut self, source: &LinkType, destination: &LinkType) {
        self.managed_links
            .retain(|_, link| link.source != *source || link.destination != *destination)
    }

    pub fn managed_link_remove_for_type(&mut self, id: LinkType) {
        self.managed_links
            .retain(|_, link| link.source != id && link.destination != id);
    }

    pub fn managed_link_bound(&mut self, id: Ulid, link_id: Ulid, pw_id: u32) {
        // Check pending syncs first
        if let Some(pending) = self
            .pending_link_syncs
            .iter_mut()
            .find(|p| p.parent_id == id)
        {
            pending.bound_ids.insert(link_id, pw_id);
            return;
        }

        // Sync already completed, group is in managed_links
        if let Some(link) = self.managed_links.get_mut(&id) {
            for port in PortLocation::iter() {
                if let Some(port) = &mut link.links[port]
                    && port.internal_id == link_id
                {
                    port.pw_id = Some(pw_id);
                    self.unmanaged_links.remove(&pw_id);
                    break;
                }
            }
            self.managed_link_ready_check(id);
        }
    }

    /// Called when a link creation error occurs (e.g., "link already exists")
    /// This notifies the sender so the caller doesn't hang waiting for a response
    pub fn managed_link_error(&mut self, parent_id: Ulid, link_id: Ulid) {
        // First, check if this is a pending link, if so, flag it.
        let mut iter = self.pending_link_syncs.iter();
        if let Some(idx) = iter.position(|p| p.parent_id == parent_id) {
            let pending = self.pending_link_syncs.remove(idx);
            warn!("Link creation failed while pending: {}", link_id);
            if let Some(sender) = pending.group.ready_sender {
                let _ = sender.send(());
            }
            return;
        }

        // Already promoted to managed_links (e.g. error on a subsequent port)
        if let Some(link) = self.managed_links.get_mut(&parent_id) {
            for port in PortLocation::iter() {
                if let Some(port) = &link.links[port]
                    && port.internal_id == link_id
                {
                    debug!("Removing failed link {} from parent {}", link_id, parent_id);
                }
            }

            if let Some(sender) = link.ready_sender.take() {
                warn!("Link creation failed for parent {}", parent_id);
                let _ = sender.send(());
            } else {
                warn!("Link creation failed for parent {}", parent_id);
                if let Some(link) = self.managed_links.remove(&parent_id) {
                    let _ = self.callback_tx.send(PipewireReceiver::ManagedLinkDropped(
                        link.source,
                        link.destination,
                    ));
                }
            }
        }
    }

    pub fn managed_link_ready_check(&mut self, id: Ulid) {
        if let Some(link) = self.managed_links.get_mut(&id) {
            if link.ready_sender.is_none() {
                return;
            }

            if !link.ports_configured() {
                // This port isn't even configured (eh?)
                error!("Link Missing Port Configuration: {}", id);
                return;
            }

            if !link.all_bound() {
                return;
            }

            // Ok, we get here, we're ready
            let sender = link.ready_sender.take();
            let _ = sender.unwrap().send(());
        }
    }
}

pub(crate) fn create_port_link(
    core: &CoreRc,
    parent_id: Ulid,
    map: &mut ManagedLinkMap,
    listener_store: Weak<RefCell<Store>>,
) -> Result<()> {
    let id = map.internal_id;
    let (src_node, src_port) = map.source_port;
    let (dest_node, dest_port) = map.destination_port;

    let listener_info_store = listener_store;
    let link = core
        .create_object::<Link>(
            "link-factory",
            &properties! {
                *LINK_OUTPUT_NODE => src_node.to_string(),
                *LINK_OUTPUT_PORT => src_port.to_string(),
                *LINK_INPUT_NODE => dest_node.to_string(),
                *LINK_INPUT_PORT => dest_port.to_string(),
                *OBJECT_LINGER => "false",
                *NODE_PASSIVE => "false",
            },
        )
        .map_err(|e| anyhow!("Failed to create link: {}", e))?;

    let listener_bound_store = listener_info_store.clone();
    let listener_error_store = listener_info_store.clone();
    let proxy_listener = link
        .upcast_ref()
        .add_listener_local()
        .bound(move |pw_id| {
            // The link is now bound and has an ID, notify the store
            if let Some(store) = listener_bound_store.upgrade() {
                store.borrow_mut().managed_link_bound(parent_id, id, pw_id);
            }
        })
        .error(move |seq, res, message| {
            log::error!(
                "[Link {}:{}] Link proxy error! seq={}, res={}, message={}",
                parent_id,
                id,
                seq,
                res,
                message
            );
            // Notify the store about the error so the sender doesn't hang
            if let Some(store) = listener_error_store.upgrade() {
                store.borrow_mut().managed_link_error(parent_id, id);
            }
        })
        .register();

    let listener_done_store = listener_info_store.clone();
    let listener_done_core = core.clone();
    let state_done = Cell::new(false);
    let link_listener = link
        .add_listener_local()
        .info(move |info| {
            if info.change_mask().contains(LinkChangeMask::STATE) {
                if state_done.get() {
                    return;
                }
                if matches!(info.state(), LinkState::Active | LinkState::Paused) {
                    state_done.set(true);

                    if let Some(store) = listener_done_store.upgrade() {
                        let seq = listener_done_core.sync(0).expect("core sync failed");
                        store
                            .borrow_mut()
                            .set_pending_link_done(parent_id, id, seq.raw());
                    }
                }
            }
            //if matches!(info.state(), LinkState::Error(e) | LinkState::Unlinked) {}
        })
        .register();

    map._link = Some(link);
    map._proxy_listener = Some(proxy_listener);
    map._info_listener = Some(link_listener);

    Ok(())
}
