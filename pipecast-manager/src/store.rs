use std::cell::RefCell;
use oneshot::Sender;
use pipewire::filter::{Filter, FilterListener, FilterPort};
use pipewire::node::{Node, NodeListener};
use pipewire::properties::Properties;
use pipewire::proxy::ProxyListener;
use pipewire::spa::param::ParamType;
use std::collections::HashMap;
use std::rc::Rc;
use log::debug;
use pipewire::link::Link;
use ulid::Ulid;
use crate::{FilterValue, LinkType};
use crate::manager::FilterData;

pub struct Store {
    // core: Core,
    //
    nodes: HashMap<Ulid, NodeStore>,
    filters: HashMap<Ulid, FilterStore>,
    links: Vec<LinkStore>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            // core,
            nodes: HashMap::new(),
            filters: HashMap::new(),
            links: vec![],
        }
    }

    pub fn add_node(&mut self, node: NodeStore) {
        debug!("[{}] Device Added to Store, waiting for data", &node.id);
        self.nodes.insert(node.id, node);
    }
    
    pub fn get_node(&self, id: Ulid) -> Option<&NodeStore> {
        self.nodes.get(&id)
    }

    pub fn node_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let node = self.nodes.get_mut(&id).expect("Broke");
        node.pw_id.replace(pw_id);
        
        self.node_check_ready(id);
    }

    pub fn node_request_ports(&self, id: Ulid) {
        let node = self.nodes.get(&id).expect("Broke");
        node.proxy
            .enum_params(0, Some(ParamType::PortConfig), 0, u32::MAX);
    }

    pub fn node_set_ports(&mut self, id: Ulid, is_input: bool, ports: Vec<u32>) {
        let node = self.nodes.get_mut(&id).expect("Broke");
        if is_input {
            node.input_ports = ports;
        } else {
            node.output_ports = ports;
        }
    }

    pub fn node_ports_ready(&mut self, id: Ulid) {
        let node = self.nodes.get_mut(&id).expect("Broke");
        node.ports_ready = true;
        self.node_check_ready(id);
    }

    pub fn node_check_ready(&mut self, id: Ulid) {
        let node = self
            .nodes
            .get_mut(&id)
            .expect("Attempted to lookup non-existing node!");
        
        if node.ports_ready && node.pw_id.is_some() {
            if let Some(sender) = node.ready_sender.take() {
                debug!("[{}] Device Ready, sending callback", &id);
                let _ = sender.send(());            
            }
        }
    }

    pub fn add_filter(&mut self, filter: FilterStore) {
        debug!("[{}] Filter Added to Store", &filter.id);
        self.filters.insert(filter.id, filter);
    }
    
    pub fn get_filter(&self, id: Ulid) -> Option<&FilterStore> {
        self.filters.get(&id)
    }

    pub fn filter_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let filter = self.filters.get_mut(&id).expect("Broke");
        filter.pw_id = Some(pw_id);

        if let Some(sender) = filter.ready_sender.take() {
            let _ = sender.send(());
        }
    }

    pub fn filter_set_parameter(&mut self, id: Ulid, key: u32, value: FilterValue) {
        let filter = self.filters.get_mut(&id).expect("Broke!");
        filter.data.borrow_mut().callback.set_property(key, value);
    }
    
    pub fn add_link(&mut self, link: LinkStore) {
        self.links.push(link);
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
    pub(crate) pw_id: Option<u32>,

    pub(crate) id: Ulid,
    
    pub(crate) filter_listener: FilterListener<Rc<RefCell<FilterData>>>,
  
    pub(crate) input_ports: Vec<FilterPort>,
    pub(crate) output_ports: Vec<FilterPort>,
    
    pub filter: Filter,

    pub(crate) ready_sender: Option<Sender<()>>,

    pub data: Rc<RefCell<FilterData>>,
}

pub struct LinkStore {
    pub(crate) link: Link,
    
    pub(crate) source: LinkType,
    pub(crate) src_port_id: usize,
    
    pub(crate) destination: LinkType,
    pub(crate) dest_port_id: usize,    
}