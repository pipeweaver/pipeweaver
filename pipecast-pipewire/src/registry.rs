use crate::store::Store;
use enum_map::{Enum, EnumMap};
use log::debug;
use pipewire::keys::{APP_ID, APP_NAME, APP_PROCESS_ID, AUDIO_CHANNEL, DEVICE_DESCRIPTION, DEVICE_ID, DEVICE_NAME, DEVICE_NICK, LINK_ID, LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, NODE_DESCRIPTION, NODE_ID, NODE_NAME, NODE_NICK, PORT_DIRECTION, PORT_ID, PORT_MONITOR, PORT_NAME};
use pipewire::registry::Listener;
use pipewire::registry::Registry;
use pipewire::types::ObjectType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct PipewireRegistry {
    registry: Registry,
    store: Rc<RefCell<Store>>,

    // These two need to exist, if the Listeners are dropped they simply stop working.
    registry_listener: Option<Listener>,
    registry_removal_listener: Option<Listener>,
}

impl PipewireRegistry {
    pub fn new(registry: Registry, store: Rc<RefCell<Store>>) -> Self {
        let mut registry = Self {
            registry,
            store,
            registry_listener: None,
            registry_removal_listener: None,
        };

        registry.registry_listener = Some(registry.register_listener());
        registry.registry_removal_listener = Some(registry.registry_removal_listener());

        registry
    }

    pub fn register_listener(&self) -> Listener {
        let store = self.store.clone();
        self.registry
            .add_listener_local()
            .global(
                move |global| {
                    let id = global.id;

                    let mut store = store.borrow_mut();
                    match global.type_ {
                        ObjectType::Device => {
                            if let Some(props) = global.props {
                                let nick = props.get(*DEVICE_NICK);
                                let desc = props.get(*DEVICE_DESCRIPTION);
                                let name = props.get(*DEVICE_NAME);

                                // Create the Device
                                let device = RegistryDevice::new(nick, desc, name);
                                store.unmanaged_device_add(id, device);
                            }
                        }
                        ObjectType::Node => {
                            if let Some(props) = global.props {
                                let device = props.get(*DEVICE_ID);
                                let nick = props.get(*NODE_NICK);
                                let desc = props.get(*NODE_DESCRIPTION);
                                let name = props.get(*NODE_NAME);

                                // Can we attach this to a Device?
                                if let Some(device) = device.and_then(|s| s.parse::<u32>().ok()) {
                                    if let Some(device) = store.unmanaged_device_get(device) {
                                        let node = RegistryNode::new(nick, desc, name);
                                        device.add_node(id, node);
                                    }
                                }
                            }
                        }

                        ObjectType::Port => {
                            if let Some(props) = global.props {
                                let node_id = props.get(*NODE_ID);
                                let pid = props.get(*PORT_ID);
                                let name = props.get(*PORT_NAME);
                                let channel = props.get(*AUDIO_CHANNEL);
                                let direction = props.get(*PORT_DIRECTION);
                                let is_monitor = props.get(*PORT_MONITOR);

                                // Realistically, the only field that can be missing which we can infer
                                // a default from would be 'is_monitor'
                                if node_id.is_none() || pid.is_none() || name.is_none() || channel.is_none() || direction.is_none() {
                                    return;
                                }

                                // Ok, we can unwrap these vars
                                let name = name.unwrap();
                                let channel = channel.unwrap();

                                // Unwrap the Port Direction. Pipewire also supports 'notify' and
                                // 'control' ports, if we run into either of those, they're not
                                // useful here, so we'll ignore the port entirely
                                let direction = match direction.unwrap() {
                                    "in" => Direction::In,
                                    "out" => Direction::Out,
                                    _ => return
                                };

                                // Unwrap the Monitor boolean. This should be set, but if it's not
                                // we'll assume it's NOT a monitor port.
                                let is_monitor = if let Some(monitor) = is_monitor {
                                    monitor.parse::<bool>().unwrap_or_default()
                                } else {
                                    false
                                };

                                // We need to extract the NodeID and PortID from the data..
                                if let Some(node_id) = node_id.and_then(|s| s.parse::<u32>().ok()) {
                                    if let Some(port_id) = pid.and_then(|s| s.parse::<u32>().ok()) {
                                        if let Some(node) = store.unamanged_node_get(node_id) {
                                            node.add_port(
                                                port_id,
                                                direction,
                                                RegistryPort::new(id, name, channel, is_monitor),
                                            )
                                        }
                                    }
                                }
                            }
                        }

                        ObjectType::Link => {
                            // We need to track links, to allow callbacks when links are created.
                            if let Some(props) = global.props {
                                let link_id = props.get(*LINK_ID).and_then(|s| s.parse::<u32>().ok());
                                let input_node = props.get(*LINK_INPUT_NODE).and_then(|s| s.parse::<u32>().ok());
                                let input_port = props.get(*LINK_INPUT_PORT).and_then(|s| s.parse::<u32>().ok());
                                let output_node = props.get(*LINK_OUTPUT_NODE).and_then(|s| s.parse::<u32>().ok());
                                let output_port = props.get(*LINK_OUTPUT_PORT).and_then(|s| s.parse::<u32>().ok());

                                // All these variables need to be set..
                                if link_id.is_none() || input_node.is_none() || input_port.is_none() || output_node.is_none() || output_port.is_none() {
                                    return;
                                }
                                store.unmanaged_link_add(link_id.unwrap(),
                                                         RegistryLink {
                                                             input_node: input_node.unwrap(),
                                                             input_port: input_port.unwrap(),
                                                             output_node: output_node.unwrap(),
                                                             output_port: output_port.unwrap(),
                                                         });
                            }
                        }

                        // ObjectType::Client => {}
                        // ObjectType::ClientEndpoint => {}
                        // ObjectType::ClientNode => {}
                        // ObjectType::ClientSession => {}
                        // ObjectType::Core => {}
                        // ObjectType::Device => {}
                        // ObjectType::Endpoint => {}
                        // ObjectType::EndpointLink => {}
                        // ObjectType::EndpointStream => {}
                        // ObjectType::Factory => {}
                        // ObjectType::Link => {}
                        // ObjectType::Metadata => {}
                        // ObjectType::Module => {}
                        // ObjectType::Node => {}
                        // ObjectType::Port => {}
                        // ObjectType::Profiler => {}
                        // ObjectType::Registry => {}
                        // ObjectType::Session => {}
                        // ObjectType::Other(_) => {}

                        _ => {
                            debug!("Unmonitored Global Type: {} - {}", global.type_, global.id);
                        }
                    }
                }
            )
            .register()
    }

    pub fn registry_removal_listener(&self) -> Listener {
        self.registry
            .add_listener_local()
            .global_remove(|id| {
                //debug!("Object Removed: {}", id);
            })
            .register()
    }
}

#[derive(Debug)]
pub(crate) struct RegistryDevice {
    nickname: Option<String>,
    description: Option<String>,
    name: Option<String>,

    pub(crate) nodes: HashMap<u32, RegistryNode>,
}

impl RegistryDevice {
    pub fn new(nickname: Option<&str>, description: Option<&str>, name: Option<&str>) -> Self {
        let nickname = nickname.map(|nickname| nickname.to_string());
        let description = description.map(|description| description.to_string());
        let name = name.map(|name| name.to_string());

        Self {
            nickname,
            description,
            name,

            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: u32, node: RegistryNode) {
        self.nodes.insert(id, node);
    }
}

#[derive(Debug, Enum)]
pub(crate) enum Direction {
    In,
    Out,
}

#[derive(Debug)]
pub(crate) struct RegistryNode {
    pub nickname: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,

    pub ports: EnumMap<Direction, HashMap<u32, RegistryPort>>,
}

impl RegistryNode {
    pub(crate) fn add_port(&mut self, id: u32, direction: Direction, port: RegistryPort) {
        self.ports[direction].insert(id, port);
    }
}

impl RegistryNode {
    pub fn new(nickname: Option<&str>, description: Option<&str>, name: Option<&str>) -> Self {
        let nickname = nickname.map(|nickname| nickname.to_string());
        let description = description.map(|description| description.to_string());
        let name = name.map(|name| name.to_string());

        Self {
            nickname,
            description,
            name,

            ports: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RegistryPort {
    pub global_id: u32,
    pub name: String,
    pub channel: String,
    pub is_monitor: bool,
}

impl RegistryPort {
    pub fn new(id: u32, name: &str, channel: &str, is_monitor: bool) -> Self {
        let name = name.to_string();
        let channel = channel.to_string();

        Self {
            global_id: id,
            name,
            channel,
            is_monitor,
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RegistryLink {
    pub input_node: u32,
    pub input_port: u32,
    pub output_node: u32,
    pub output_port: u32,
}
