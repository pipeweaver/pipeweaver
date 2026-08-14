//! Lifecycle of the links we create. This is the fiddliest bit of the
//! store: a "link" is really a pair of per-channel pipewire links (left and
//! right) that both need to bind and get a pipewire id before we consider
//! the managed link ready, so most of this module is bookkeeping around
//! `pending_link_syncs` until that's true.

use super::{LinkStore, LinkStoreMap, PendingLinkSync, PortLocation, Store};
use crate::{LinkType, PipewireReceiver};
use log::{debug, error, warn};
use std::collections::HashMap;
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

    pub fn managed_link_add(&mut self, id: Ulid, group: LinkStore) {
        self.managed_links.insert(id, group);
    }

    pub fn add_pending_link(&mut self, parent_id: Ulid, group: LinkStore) {
        self.pending_link_syncs.push(PendingLinkSync {
            parent_id,
            group,
            bound_ids: HashMap::new(),
        });
    }

    pub fn get_next_pending_link(&mut self, seq: i32) -> Option<&mut LinkStoreMap> {
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

            // Iterate over all the links, check if they all have a pw_id assigned
            for port in PortLocation::iter() {
                if let Some(port) = &link.links[port] {
                    if port.pw_id.is_none() {
                        return;
                    }
                } else {
                    // This port isn't even configured (eh?)
                    error!("Link Missing Port Configuration: {}", id);
                    return;
                }
            }

            // Ok, we get here, we're ready
            let sender = link.ready_sender.take();
            let _ = sender.unwrap().send(());
        }
    }
}
