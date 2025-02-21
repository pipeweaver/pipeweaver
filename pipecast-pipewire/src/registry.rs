use crate::store::Store;
use log::debug;
use pipewire::keys::{
    APP_ID, APP_NAME, APP_PROCESS_ID, AUDIO_CHANNEL, DEVICE_DESCRIPTION, DEVICE_ID, DEVICE_NAME,
    DEVICE_NICK, NODE_DESCRIPTION, NODE_ID, NODE_NAME, NODE_NICK, PORT_DIRECTION, PORT_MONITOR,
    PORT_NAME,
};
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
            .global({
                move |global| {
                    let mut store = store.borrow_mut();
                    match global.type_ {
                        ObjectType::Device => {
                            if let Some(props) = global.props {
                                let nick = props.get(*DEVICE_NICK);
                                let desc = props.get(*DEVICE_DESCRIPTION);
                                let name = props.get(*DEVICE_NAME);

                                // Create the Device
                                let device = RegistryDevice::new(nick, desc, name);

                                // Register it with the store
                                store.add_unmanaged_device(global.id, device);

                                debug!("{:#?}", store.get_unmanaged_devices())
                            }
                        }
                        ObjectType::Node => {
                            if let Some(props) = global.props {
                                let device = props.get(*DEVICE_ID);
                                let nick = props.get(*NODE_NICK);
                                let desc = props.get(*NODE_DESCRIPTION);
                                let name = props.get(*NODE_NAME);

                                // Can we attach this to a Device?
                                if let Some(device_id) = device {
                                    if let Ok(device_id) = device_id.parse::<u32>() {
                                        let node_id = global.id;

                                        // Check whether this NodeId is already managed
                                        if !store.is_managed_node(node_id) {
                                            let device = store.get_unmanaged_device(device_id);

                                            // Get the RegistryDevice for this device_id
                                            if let Some(device) = device {
                                                let node = RegistryNode::new(nick, desc, name);
                                                device.add_node(global.id, node);
                                            }
                                        }
                                    }
                                }

                                debug!("{:#?}", store.get_unmanaged_devices())
                            }
                        }

                        ObjectType::Port => {
                            if let Some(props) = global.props {
                                let node_id = props.get(*NODE_ID);
                                let name = props.get(*PORT_NAME);
                                let channel = props.get(*AUDIO_CHANNEL);
                                let direction = props.get(*PORT_DIRECTION);
                                let is_monitor = props.get(*PORT_MONITOR);

                                // If we don't have sufficient info to match this, ignore it.
                                if name.is_none() || channel.is_none() || direction.is_none() {
                                    return;
                                }

                                // Ok, we can unwrap these vars
                                let name = name.unwrap();
                                let channel = channel.unwrap();
                                let direction = direction.unwrap();
                                let is_monitor = if let Some(monitor) = is_monitor {
                                    monitor.parse::<bool>().unwrap_or_default()
                                } else {
                                    false
                                };

                                if let Some(node) = node_id {
                                    if let Ok(node_id) = node.parse::<u32>() {
                                        // Get the Unmanaged node
                                        if let Some(node) = store.get_unmanaged_node(node_id) {
                                            node.add_port(
                                                global.id,
                                                RegistryPort::new(
                                                    name, channel, direction, is_monitor,
                                                ),
                                            );
                                        }
                                    }
                                }
                                debug!("{:#?}", store.get_unmanaged_devices())
                            }
                        }

                        ObjectType::Link => {}

                        _ => {
                            debug!("Unmonitored Global Type: {} - {}", global.type_, global.id);
                        } // ObjectType::Client => {}
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
                    }
                }
            })
            .register()
    }

    pub fn registry_removal_listener(&self) -> Listener {
        self.registry
            .add_listener_local()
            .global_remove(|id| {
                debug!("Object Removed: {}", id);
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

    pub fn should_export(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub(crate) struct RegistryNode {
    nickname: Option<String>,
    description: Option<String>,
    name: Option<String>,

    ports: HashMap<u32, RegistryPort>,
}

impl RegistryNode {
    pub(crate) fn add_port(&mut self, id: u32, port: RegistryPort) {
        self.ports.insert(id, port);
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

            ports: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RegistryPort {
    name: String,
    channel: String,
    direction: String,
    is_monitor: bool,
}

impl RegistryPort {
    pub fn new(name: &str, channel: &str, direction: &str, is_monitor: bool) -> Self {
        let name = name.to_string();
        let channel = channel.to_string();
        let direction = direction.to_string();

        Self {
            name,
            channel,
            direction,
            is_monitor,
        }
    }
}

pub(crate) struct RegistryLink {}
