use crate::manager::FilterData;
use crate::registry::{Direction, RegistryDevice, RegistryLink, RegistryNode};
use crate::{FilterValue, LinkType, PipecastNode};
use log::debug;
use parking_lot::RwLock;
use pipewire::filter::{Filter, FilterListener, FilterPort};
use pipewire::link::{Link, LinkListener};
use pipewire::node::{Node, NodeListener};
use pipewire::properties::Properties;
use pipewire::proxy::ProxyListener;
use pipewire::spa::param::ParamType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tokio::sync::oneshot::Sender;
use ulid::Ulid;

pub struct Store {
    managed_nodes: HashMap<Ulid, NodeStore>,
    managed_filters: HashMap<Ulid, FilterStore>,
    managed_links: HashMap<Ulid, LinkStore>,

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

    pub fn unamanged_node_get(&mut self, id: u32) -> Option<&mut RegistryNode> {
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

    pub fn node_set_ports(&mut self, id: Ulid, is_input: bool, ports: Vec<u32>) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        if is_input {
            node.input_ports = ports;
        } else {
            node.output_ports = ports;
        }
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
                let _ = sender.send(());
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
            let _ = sender.send(());
        }
    }

    pub fn filter_set_parameter(&mut self, id: Ulid, key: u32, value: FilterValue) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke!");
        filter.data.write().callback.set_property(key, value);
    }

    pub fn link_add(&mut self, id: Ulid, link: LinkStore) {
        self.managed_links.insert(id, link);
    }

    pub fn link_ready(&mut self, id: Ulid) {
        if let Some(link) = self.managed_links.get_mut(&id) {
            let sender = link.ready_sender.take();
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
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

// This works, but is horrifically messy..
// TODO: CLEAN THIS UP.
pub(crate) struct NodeStore {
    pub(crate) pw_id: Option<u32>,

    pub(crate) id: Ulid,
    pub(crate) props: Properties,

    pub(crate) proxy: Node,
    pub(crate) proxy_listener: ProxyListener,
    pub(crate) listener: NodeListener,

    // TODO: These should be a map of Position -> Id
    pub(crate) ports_ready: bool,
    pub(crate) input_ports: Vec<u32>,
    pub(crate) output_ports: Vec<u32>,

    pub(crate) ready_sender: Option<Sender<()>>,
}

pub struct FilterStore {
    /// The Pipewire Node ID for this Filter
    pub(crate) pw_id: Option<u32>,

    pub(crate) _listener: FilterListener<Rc<RwLock<FilterData>>>,

    /// The PipeCast Ulid Identifier
    pub(crate) id: Ulid,

    /// Details of the ports assigned to this filter
    pub(crate) input_ports: Rc<RefCell<Vec<FilterPort>>>,
    pub(crate) output_ports: Rc<RefCell<Vec<FilterPort>>>,

    /// These two fields need to exist purely to prevent the filter and the listener from
    /// being dropped, they're never directly accessed, they're just a store.
    pub(crate) _filter: Filter,

    /// The 'Ready Sender' is called once the filter is setup and ready-to-go
    pub(crate) ready_sender: Option<Sender<()>>,

    /// The Data related to the filter, including the sample processing callback
    pub data: Rc<RwLock<FilterData>>,
}

pub struct LinkStore {
    pub(crate) link: Link,
    pub(crate) _listener: LinkListener,

    pub(crate) source: LinkType,
    pub(crate) src_port_id: usize,

    pub(crate) destination: LinkType,
    pub(crate) dest_port_id: usize,

    pub(crate) ready_sender: Option<Sender<()>>,
}

// #[derive(Debug)]
// pub struct LinkListener {
//     link: RegistryLink,
//     listener: oneshot::Sender<()>,
// }