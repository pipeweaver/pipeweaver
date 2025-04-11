use crate::manager::FilterData;
use crate::registry::{Direction, RegistryDevice, RegistryLink, RegistryNode};
use crate::{FilterValue, LinkType, PipecastNode};
use anyhow::bail;
use enum_map::{Enum, EnumMap};
use log::{debug, error};
use parking_lot::RwLock;
use pipewire::filter::{Filter, FilterListener, FilterPort};
use pipewire::link::{Link, LinkListener};
use pipewire::node::{Node, NodeListener};
use pipewire::properties::Properties;
use pipewire::proxy::ProxyListener;
use pipewire::spa::param::ParamType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::sync::oneshot::Sender;
use ulid::Ulid;

pub struct Store {
    managed_nodes: HashMap<Ulid, NodeStore>,
    managed_filters: HashMap<Ulid, FilterStore>,
    managed_links: HashMap<Ulid, LinkGroupStore>,

    unmanaged_devices: HashMap<u32, RegistryDevice>,
    unmanaged_links: HashMap<u32, RegistryLink>,

    link_listeners: Vec<LinkListener>,

}

impl Store {
    pub fn new() -> Self {
        Self {
            // core,
            managed_nodes: HashMap::new(),
            managed_filters: HashMap::new(),
            managed_links: HashMap::new(),

            unmanaged_devices: HashMap::new(),
            unmanaged_links: HashMap::new(),

            link_listeners: vec![],
        }
    }

    pub fn is_managed_node(&self, id: u32) -> bool {
        for node in self.managed_nodes.values() {
            if let Some(managed_id) = node.pw_id {
                if managed_id == id {
                    return true;
                }
            }
        }
        false
    }

    pub fn unmanaged_device_add(&mut self, id: u32, device: RegistryDevice) {
        // Only add this if the node isn't already managed
        if self.is_managed_node(id) {
            return;
        }
        self.unmanaged_devices.insert(id, device);
    }

    pub fn unmanaged_devices_get(&self) -> &HashMap<u32, RegistryDevice> {
        &self.unmanaged_devices
    }

    pub fn unmanaged_device_get(&mut self, id: u32) -> Option<&mut RegistryDevice> {
        self.unmanaged_devices.get_mut(&id)
    }

    pub fn unamanged_node_get_mut(&mut self, id: u32) -> Option<&mut RegistryNode> {
        // We need to find this node inside the unmanaged devices..
        for device in self.unmanaged_devices.values_mut() {
            for (node_id, node) in &mut device.nodes {
                if *node_id == id {
                    return Some(node);
                }
            }
        }
        None
    }

    pub fn unamanged_node_get(&self, id: u32) -> Option<&RegistryNode> {
        // We need to find this node inside the unmanaged devices..
        for device in self.unmanaged_devices.values() {
            for (node_id, node) in &device.nodes {
                if *node_id == id {
                    return Some(node);
                }
            }
        }
        None
    }

    pub fn unmanaged_link_add(&mut self, id: u32, link: RegistryLink) {
        self.unmanaged_links.insert(id, link);
    }

    pub fn node_add(&mut self, node: NodeStore) {
        debug!("[{}] Device Added to Store, waiting for data", &node.id);
        self.managed_nodes.insert(node.id, node);
    }

    pub fn node_get(&self, id: Ulid) -> Option<&NodeStore> {
        self.managed_nodes.get(&id)
    }

    pub fn node_remove(&mut self, id: Ulid) {
        // This should cause pipewire to drop the node as soon as it goes out of scope. We don't
        // check for things like links here, PW will clean them up, so upstream should manage 
        // anything extra.
        if self.managed_nodes.contains_key(&id) {
            self.managed_nodes.remove(&id);
        }
    }

    pub fn node_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        node.pw_id.replace(pw_id);

        self.node_check_ready(id);
    }

    pub fn node_request_ports(&self, id: Ulid) {
        let node = self.managed_nodes.get(&id).expect("Broke");
        node.proxy
            .enum_params(0, Some(ParamType::PortConfig), 0, u32::MAX);
    }

    pub fn node_add_port(&mut self, id: Ulid, location: PortLocation, port_id: u32) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        node.port_map[location] = Some(port_id);

        for location in PortLocation::iter() {
            if node.port_map[location].is_none() {
                return;
            }
        }

        // If we get here, all our ports have been set, trigger the ready event
        self.node_ports_ready(id);
    }

    pub fn node_ports_ready(&mut self, id: Ulid) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        node.ports_ready = true;
        self.node_check_ready(id);
    }

    pub fn node_check_ready(&mut self, id: Ulid) {
        let node = self
            .managed_nodes
            .get_mut(&id)
            .expect("Attempted to lookup non-existing node!");

        if node.ports_ready && node.pw_id.is_some() {
            if let Some(sender) = node.ready_sender.take() {
                debug!("[{}] Device Ready, sending callback", &id);
                if let Some(sender) = sender {
                    let _ = sender.send(());
                }
            }
        }
    }

    pub fn filter_add(&mut self, filter: FilterStore) {
        debug!("[{}] Filter Added to Store", &filter.id);
        self.managed_filters.insert(filter.id, filter);
    }

    pub fn filter_get(&self, id: Ulid) -> Option<&FilterStore> {
        self.managed_filters.get(&id)
    }

    pub fn filter_remove(&mut self, filter: Ulid) {
        self.link_remove_for_type(LinkType::Filter(filter));
        self.managed_filters.remove(&filter);
    }

    pub fn filter_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        filter.pw_id = Some(pw_id);

        if let Some(sender) = filter.ready_sender.take() {
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
        }
    }

    pub fn filter_set_parameter(&mut self, id: Ulid, key: u32, value: FilterValue) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke!");
        filter.data.write().callback.set_property(key, value);
    }

    pub fn link_add_group(&mut self, id: Ulid, group: LinkGroupStore) {
        self.managed_links.insert(id, group);
    }

    pub fn link_ready(&mut self, id: Ulid, link_id: Ulid, pw_id: u32) {
        if let Some(link) = self.managed_links.get_mut(&id) {
            for port in PortLocation::iter() {
                if let Some(port) = &mut link.links[port] {
                    if port.internal_id == link_id {
                        port.pw_id = Some(pw_id);
                    }
                }
            }
        }

        // Regardless of what's happened here, perform a ready check on the parent
        self.link_ready_check(id);
    }

    pub fn link_ready_check(&mut self, id: Ulid) {
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

    pub fn link_remove(&mut self, source: LinkType, destination: LinkType) {
        debug!("Removing Link..");
        self.managed_links.retain(|_, link| {
            link.source != source || link.destination != destination
        })
    }

    pub fn link_remove_for_type(&mut self, id: LinkType) {
        debug!("Dropping Link Type..");
        self.managed_links.retain(|_, link| {
            link.source != id && link.destination != id
        });
    }

    /// This function returns a list of nodes which can be linked to and from inside PipeCast
    pub fn get_usable_nodes(&self) -> Vec<PipecastNode> {
        // Firstly, iterate over the devices, we need to peek into all of them to check nodes..
        let mut pipecast_nodes: Vec<PipecastNode> = Vec::new();

        for device in self.unmanaged_devices.values() {
            for (node_id, node) in &device.nodes {
                // We need to count the number of non-monitor ports on the input and output
                let mut in_count = 0;
                let mut out_count = 0;
                for port in node.ports[Direction::In].values() {
                    if !port.is_monitor {
                        in_count += 1;
                    }
                }
                for port in node.ports[Direction::Out].values() {
                    if !port.is_monitor {
                        out_count += 1;
                    }
                }

                // In this instance, if we have more than 2 ports we can't easily link them
                // together effectively, so we'll ignore this node, we'll also ignore it if there
                // are NO ports at all!
                if in_count > 2 || out_count > 2 || (in_count == 0 && out_count == 0) {
                    continue;
                }

                // We get here, this node should be usable
                pipecast_nodes.push(PipecastNode {
                    node_id: *node_id,
                    name: node.name.clone(),
                    nickname: node.nickname.clone(),
                    description: node.description.clone(),
                    inputs: in_count,
                    outputs: out_count,
                });
            }
        }
        pipecast_nodes
    }
}

pub(crate) struct NodeStore {
    pub(crate) pw_id: Option<u32>,

    pub(crate) id: Ulid,
    pub(crate) props: Properties,

    pub(crate) proxy: Node,
    pub(crate) proxy_listener: ProxyListener,
    pub(crate) listener: NodeListener,

    // Nodes will always have inputs and outputs which directly link together, so we
    // don't need to track each side, we just need the ID and Location
    pub(crate) port_map: EnumMap<PortLocation, Option<u32>>,
    pub(crate) ports_ready: bool,

    pub(crate) ready_sender: Option<Option<Sender<()>>>,
}

pub struct FilterStore {
    /// The Pipewire Node ID for this Filter
    pub(crate) pw_id: Option<u32>,

    pub(crate) _listener: FilterListener<Rc<RwLock<FilterData>>>,

    /// The PipeCast Ulid Identifier
    pub(crate) id: Ulid,

    // This maintains a general port map of location -> index
    pub(crate) port_map: EnumMap<Direction, EnumMap<PortLocation, u32>>,

    /// Details of the ports assigned to this filter
    pub(crate) input_ports: Rc<RefCell<Vec<FilterPort>>>,
    pub(crate) output_ports: Rc<RefCell<Vec<FilterPort>>>,

    /// These two fields need to exist purely to prevent the filter and the listener from
    /// being dropped, they're never directly accessed, they're just a store.
    pub(crate) _filter: Filter,

    /// The 'Ready Sender' is called once the filter is setup and ready-to-go
    pub(crate) ready_sender: Option<Option<Sender<()>>>,

    /// The Data related to the filter, including the sample processing callback
    pub data: Rc<RwLock<FilterData>>,
}

pub struct LinkGroupStore {
    pub(crate) source: LinkType,
    pub(crate) destination: LinkType,

    pub(crate) links: EnumMap<PortLocation, Option<LinkStoreMap>>,

    pub(crate) ready_sender: Option<Sender<()>>,
}

pub struct LinkStoreMap {
    pub(crate) pw_id: Option<u32>,

    /// An internal ID so we can find this link
    pub(crate) internal_id: Ulid,

    /// Variables needed to keep this link alive
    pub(crate) link: Link,
    pub(crate) _listener: LinkListener,

    /// Internal Port Index Mapping
    pub(crate) source_port_id: u32,
    pub(crate) destination_port_id: u32,
}

#[derive(Debug, Enum, EnumIter, Copy, Clone, PartialEq)]
pub(crate) enum PortLocation {
    LEFT,
    RIGHT,
}

impl Display for PortLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PortLocation::LEFT => write!(f, "FL"),
            PortLocation::RIGHT => write!(f, "FR")
        }
    }
}

impl FromStr for PortLocation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FL" => Ok(Self::LEFT),
            "FR" => Ok(Self::RIGHT),
            _ => bail!("Unknown Channel")
        }
    }
}

// #[derive(Debug)]
// pub struct LinkListener {
//     link: RegistryLink,
//     listener: oneshot::Sender<()>,
// }