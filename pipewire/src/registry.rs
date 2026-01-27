use crate::NodeTarget;
use crate::default_device::DefaultDefinition;
use crate::store::{Store, TargetType};
use anyhow::{anyhow, bail};
use enum_map::{Enum, EnumMap};
use log::debug;
use pipewire::client::{Client, ClientChangeMask, ClientListener};
use pipewire::keys::{
    ACCESS, APP_NAME, APP_PROCESS_BINARY, AUDIO_CHANNEL, CLIENT_ID, DEVICE_DESCRIPTION, DEVICE_ID,
    DEVICE_NAME, DEVICE_NICK, FACTORY_NAME, FACTORY_TYPE_NAME, FACTORY_TYPE_VERSION,
    LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, MEDIA_CLASS, MEDIA_NAME,
    MODULE_ID, NODE_DESCRIPTION, NODE_ID, NODE_NAME, NODE_NICK, OBJECT_SERIAL, PORT_DIRECTION,
    PORT_ID, PORT_MONITOR, PORT_NAME, PROTOCOL, SEC_GID, SEC_PID, SEC_UID,
};
use pipewire::metadata::{Metadata, MetadataListener};
use pipewire::node::{Node, NodeChangeMask, NodeListener};
use pipewire::registry::Listener;
use pipewire::registry::Registry;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::Value::Bool;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{Value, ValueArray};
use pipewire::spa::sys::{SPA_PARAM_Props, SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils::dict::DictRef;
use pipewire::types::ObjectType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct PipewireRegistry {
    registry: Rc<RefCell<Registry>>,
    store: Rc<RefCell<Store>>,

    // These two need to exist, if the Listeners are dropped they simply stop working.
    registry_listener: Option<Listener>,
    registry_removal_listener: Option<Listener>,
}

impl PipewireRegistry {
    pub fn new(registry: Registry, store: Rc<RefCell<Store>>) -> Self {
        let mut registry = Self {
            registry: Rc::new(RefCell::new(registry)),
            store,
            registry_listener: None,
            registry_removal_listener: None,
        };

        registry.registry_listener = Some(registry.register_listener());
        registry.registry_removal_listener = Some(registry.registry_removal_listener());

        registry
    }

    pub fn register_listener(&self) -> Listener {
        let local_store = Rc::downgrade(&self.store);
        let listener_store = Rc::downgrade(&self.store);
        let registry = self.registry.clone();

        self.registry
            .borrow()
            .add_listener_local()
            .global(move |global| {
                let id = global.id;

                // If the store has been dropped we can early-return
                let Some(local_store) = local_store.upgrade() else {
                    return;
                };
                let mut store = local_store.borrow_mut();

                match global.type_ {
                    ObjectType::Device => {
                        if let Some(props) = global.props {
                            // Create the Device
                            let device = RegistryDevice::from(props);
                            store.unmanaged_device_add(id, device);
                        }
                    }

                    ObjectType::Node => {
                        if let Some(props) = global.props {
                            // If we're receiving properties for a managed node, we just need to update
                            // the internal serial number if it's present.
                            if store.is_managed_node(id) {
                                // Yes, inform the store of the new pw serial
                                if let Some(serial) = props.get(*OBJECT_SERIAL).and_then(|s| s.parse::<u32>().ok()) {
                                    store.managed_node_set_pw_serial(id, serial);
                                }
                                return;
                            }

                            if let Ok(node) = RegistryDeviceNode::try_from(props) {
                                if let Some(parent_id) = node.parent_id
                                    && let Some(device) = store.unmanaged_device_get(parent_id) {
                                    device.add_node(id);
                                }

                                // All unmanaged nodes should be handled, even if they don't have a parent
                                store.unmanaged_device_node_add(id, node);
                            } else if let Ok(mut node) = RegistryClientNode::try_from(props) {
                                if let Some(client) = store.unmanaged_client_get(node.parent_id) {
                                    let bound: Option<Node> = registry.borrow().bind(global).ok();
                                    if let Some(proxy) = bound {
                                        let param_local = listener_store.clone();
                                        let info_local = listener_store.clone();
                                        let listener = proxy
                                            .add_listener_local()
                                            .param(move |_seq, _type, _index, _next, param| {
                                                if let Some(pod) = param {
                                                    let pod = PodDeserializer::deserialize_any_from(
                                                        pod.as_bytes(),
                                                    )
                                                        .map(|(_, v)| v);

                                                    if let Ok(Value::Object(object)) = pod
                                                        && object.id == SPA_PARAM_Props
                                                    {
                                                        let prop = object
                                                            .properties
                                                            .iter()
                                                            .find(|p| p.key == SPA_PROP_channelVolumes);

                                                        if let Some(prop) = prop
                                                            && let Value::ValueArray(ValueArray::Float(value)) = &prop.value
                                                        {
                                                            let vol = if value.is_empty() {
                                                                0_f32
                                                            } else if value.len() == 1 {
                                                                *value.first().unwrap()
                                                            } else {
                                                                value
                                                                    .iter()
                                                                    .copied()
                                                                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                                                                    .unwrap()
                                                            };

                                                            let volume = (vol.cbrt() * 100.0).round() as u8;
                                                            if let Some(param_local) = param_local.upgrade() {
                                                                param_local
                                                                    .borrow_mut()
                                                                    .unmanaged_client_node_set_volume(id, volume);
                                                            }
                                                        }

                                                        let prop = object.properties.iter().find(|p| p.key == SPA_PROP_mute);
                                                        if let Some(prop) = prop && let Bool(value) = prop.value && let Some(param_local) = param_local.upgrade() {
                                                            param_local
                                                                .borrow_mut()
                                                                .unmanaged_client_node_set_mute(id, value);
                                                        }
                                                    }
                                                }
                                            })
                                            .info(move |info| {
                                                for change in info.change_mask().iter() {
                                                    if change == NodeChangeMask::PROPS
                                                        && let Some(props) = info.props()
                                                        && let Some(media) = props.get(*MEDIA_NAME)
                                                        && let Some(info_local) = info_local.upgrade() {
                                                        info_local
                                                            .borrow_mut()
                                                            .unmanaged_client_node_set_media(id, String::from(media));
                                                    }

                                                    if change == NodeChangeMask::STATE && let Some(info_local) = info_local.upgrade() {
                                                        info_local
                                                            .borrow_mut()
                                                            .unmanaged_client_node_set_state(id, info.state());
                                                    }
                                                }
                                            })
                                            .register();

                                        proxy.subscribe_params(&[ParamType::Props]);
                                        proxy.enum_params(0, None, 0, u32::MAX);
                                        node.proxy = Some(proxy);
                                        node._listener = Some(listener);
                                    }
                                    client.add_node(id);
                                    store.unmanaged_client_node_add(id, node);
                                }
                            } else {
                                // We don't know what type, or props this has, so we'll fire the id and serial to the
                                // store, and see if it wants to handle it.
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

                            if node_id.is_none() || pid.is_none() || name.is_none() || channel.is_none() || direction.is_none() {
                                return;
                            }

                            let name = name.unwrap();
                            let channel = channel.unwrap();

                            let direction = match direction.unwrap() {
                                "in" => Direction::In,
                                "out" => Direction::Out,
                                _ => return,
                            };

                            let is_monitor = if let Some(monitor) = is_monitor { monitor.parse::<bool>().unwrap_or_default() } else { false };

                            let port = RegistryPort::new(id, name, channel, is_monitor);

                            if let Some(node_id) = node_id.and_then(|s| s.parse::<u32>().ok()) && let Some(port_id) = pid.and_then(|s| s.parse::<u32>().ok()) {
                                if let Some(node) = store.unmanaged_device_node_get(node_id) {
                                    node.add_port(id, direction, port);
                                    store.unmanaged_node_update(node_id);
                                    return;
                                }
                                if let Some(node) = store.unmanaged_client_node_get(node_id) {
                                    node.add_port(port_id, direction, port);
                                    store.unmanaged_client_node_check(node_id);
                                }
                            }
                        }
                    }

                    ObjectType::Link => {
                        if let Some(props) = global.props && let Ok(link) = RegistryLink::try_from(props) {
                            store.unmanaged_link_add(id, link);
                        }
                    }

                    ObjectType::Factory => {
                        if let Some(props) = global.props && let Ok(factory) = RegistryFactory::try_from(props) {
                            store.factory_add(id, factory);
                        }
                    }

                    ObjectType::Client => {
                        if let Some(props) = global.props {
                            if let Ok(mut client) = RegistryClient::try_from(props) {
                                let proxy: Option<Client> = registry.borrow().bind(global).ok();
                                if let Some(client_proxy) = proxy {
                                    let info_local = listener_store.clone();
                                    let listener = client_proxy
                                        .add_listener_local()
                                        .info(move |info| {
                                            for change in info.change_mask().iter() {
                                                if change == ClientChangeMask::PROPS && let Some(props) = info.props() && let Some(process) = props.get(*APP_PROCESS_BINARY) && let Some(info_local) = info_local.upgrade() {
                                                    info_local
                                                        .borrow_mut()
                                                        .unmanaged_client_set_binary(id, String::from(process));
                                                }
                                            }
                                        })
                                        .register();
                                    client._listener = Some(listener);
                                    client._proxy = Some(client_proxy);

                                    store.unmanaged_client_add(id, client);
                                }
                            } else {
                                debug!("Failed to create client: {:?}", props);
                            }
                        }
                    }

                    ObjectType::Metadata => {
                        if let Some(props) = global.props && let Some(name) = props.get("metadata.name") {
                            if name == "default" {
                                let proxy: Option<Metadata> = registry.borrow().bind(global).ok();
                                if let Some(metadata) = proxy {
                                    let listen_store = listener_store.clone();
                                    let listener = metadata
                                        .add_listener_local()
                                        .property(move |subject, key, _type, value| {
                                            if key == Some("target.object") {
                                                let target = value.and_then(|s| s.parse::<u32>().ok());
                                                if let Some(listen_store) = listen_store.upgrade() {
                                                    listen_store.borrow_mut().unmanaged_client_node_set_target(subject, TargetType::Serial(target));
                                                }
                                            }
                                            if key == Some("target.node") {
                                                let target = value.and_then(|s| s.parse::<u32>().ok());
                                                if let Some(listen_store) = listen_store.upgrade() {
                                                    listen_store.borrow_mut().unmanaged_client_node_set_target(subject, TargetType::Node(target));
                                                }
                                            }
                                            if key == Some("default.audio.sink")
                                                && _type == Some("Spa:String:JSON")
                                                && let Some(val) = value
                                                && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                                                && let Some(name) = json.get("name").and_then(|v| v.as_str())
                                                && let Some(listen_store) = listen_store.upgrade() {
                                                listen_store.borrow_mut().set_default_sink(DefaultDefinition::Default(String::from(name)));
                                            }

                                            if key == Some("default.configured.audio.sink")
                                                && _type == Some("Spa:String:JSON")
                                                && let Some(val) = value
                                                && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                                                && let Some(name) = json.get("name").and_then(|v| v.as_str())
                                                && let Some(listen_store) = listen_store.upgrade() {
                                                listen_store.borrow_mut().set_default_sink(DefaultDefinition::Configured(String::from(name)));
                                            }

                                            if key == Some("default.audio.source")
                                                && _type == Some("Spa:String:JSON")
                                                && let Some(val) = value
                                                && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                                                && let Some(name) = json.get("name").and_then(|v| v.as_str())
                                                && let Some(listen_store) = listen_store.upgrade() {
                                                listen_store.borrow_mut().set_default_source(DefaultDefinition::Default(String::from(name)));
                                            }

                                            if key == Some("default.configured.audio.source")
                                                && _type == Some("Spa:String:JSON")
                                                && let Some(val) = value
                                                && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                                                && let Some(name) = json.get("name").and_then(|v| v.as_str())
                                                && let Some(listen_store) = listen_store.upgrade() {
                                                listen_store.borrow_mut().set_default_source(DefaultDefinition::Configured(String::from(name)));
                                            }
                                            0
                                        })
                                        .register();

                                    let session = MetadataStore { metadata, _listener: listener };
                                    store.set_session_proxy(session);
                                }
                            } else if name == "settings" {
                                let proxy: Option<Metadata> = registry.borrow().bind(global).ok();
                                if let Some(metadata) = proxy {
                                    let listen_store = listener_store.clone();
                                    let listener = metadata
                                        .add_listener_local()
                                        .property(move |_subject, key, _type, value| {
                                            if key == Some("clock.rate") {
                                                let clock = value.and_then(|s| s.parse::<u32>().ok());
                                                if let Some(listen_store) = listen_store.upgrade() {
                                                    listen_store.borrow_mut().announce_clock_rate(clock);
                                                }
                                            }
                                            0
                                        })
                                        .register();

                                    let settings = MetadataStore { metadata, _listener: listener };
                                    store.set_settings_proxy(settings);
                                }
                            }
                        }
                    }

                    _ => {}
                }
            })
            .register()
    }

    pub fn registry_removal_listener(&self) -> Listener {
        let store = Rc::downgrade(&self.store);
        self.registry
            .borrow()
            .add_listener_local()
            .global_remove(move |id| {
                if let Some(store) = store.upgrade() {
                    store.borrow_mut().remove_by_id(id);
                }
            })
            .register()
    }

    pub fn destroy_global(&self, id: u32) {
        self.registry.borrow().destroy_global(id);
    }
}

pub(crate) struct MetadataStore {
    pub(crate) metadata: Metadata,
    pub(crate) _listener: MetadataListener,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct RegistryFactory {
    pub(crate) object_serial: u32,
    pub(crate) module_id: u32,

    pub(crate) name: String,
    pub(crate) factory_type: ObjectType,
    pub(crate) version: u32,
}

impl TryFrom<&DictRef> for RegistryFactory {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let module_id = value
            .get(*MODULE_ID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("MODULE_ID"))?;
        let name = value
            .get(*FACTORY_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("FACTORY_NAME"))?;
        let factory_type = value
            .get(*FACTORY_TYPE_NAME)
            .ok_or_else(|| anyhow!("FACTORY_TYPE_NAME"))?;
        let version = value
            .get(*FACTORY_TYPE_VERSION)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("FACTORY_VERSION"))?;

        Ok(RegistryFactory {
            object_serial,
            module_id,
            name,
            factory_type: to_object_type(factory_type),
            version,
        })
    }
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct RegistryDevice {
    object_serial: u32,

    nickname: Option<String>,
    description: Option<String>,
    name: Option<String>,

    pub(crate) nodes: Vec<u32>,
}

impl From<&DictRef> for RegistryDevice {
    fn from(value: &DictRef) -> Self {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .expect("OBJECT_SERIAL");
        let nickname = value.get(*DEVICE_NICK).map(|s| s.to_string());
        let description = value.get(*DEVICE_DESCRIPTION).map(|s| s.to_string());
        let name = value.get(*DEVICE_NAME).map(|s| s.to_string());

        Self {
            object_serial,
            nickname,
            description,
            name,
            nodes: vec![],
        }
    }
}

impl RegistryDevice {
    pub fn add_node(&mut self, id: u32) {
        self.nodes.push(id);
    }
}

#[derive(Debug, Enum)]
pub(crate) enum Direction {
    In,
    Out,
}

#[derive(Debug)]
pub(crate) struct RegistryDeviceNode {
    pub object_serial: u32,
    pub parent_id: Option<u32>,

    pub media_class: Option<String>,
    pub is_usable: bool,

    pub nickname: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,

    pub ports: EnumMap<Direction, HashMap<u32, RegistryPort>>,
}

impl TryFrom<&DictRef> for RegistryDeviceNode {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let parent_id = value.get(*DEVICE_ID).and_then(|s| s.parse::<u32>().ok());
        let nickname = value.get(*NODE_NICK).map(|s| s.to_string());
        let description = value.get(*NODE_DESCRIPTION).map(|s| s.to_string());
        let name = value.get(*NODE_NAME).map(|s| s.to_string());
        let media_class = value.get(*MEDIA_CLASS).map(|s| s.to_string());

        // We need to match the media type here, it's only a device if it's a Sink or Source
        if let Some(media_class) = &media_class {
            if !media_class.starts_with("Audio/Source") && !media_class.starts_with("Audio/Sink") {
                bail!("Not an Audio Device Node");
            }
        } else {
            bail!("Missing Media Class");
        }

        Ok(Self {
            object_serial,
            parent_id,

            media_class,
            is_usable: false,

            nickname,
            description,
            name,
            ports: Default::default(),
        })
    }
}

impl RegistryDeviceNode {
    pub(crate) fn add_port(&mut self, id: u32, direction: Direction, port: RegistryPort) {
        self.ports[direction].insert(id, port);
    }
}

#[derive(Debug)]
#[allow(unused)]
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

#[allow(unused)]
pub(crate) struct RegistryClient {
    object_serial: u32,

    module_id: u32,
    protocol: String,
    process_id: u32,
    user_id: u32,
    group_id: u32,
    access: String,
    pub(crate) application_name: String,
    pub(crate) application_binary: Option<String>,

    pub(crate) _proxy: Option<Client>,
    pub(crate) _listener: Option<ClientListener>,

    pub(crate) nodes: Vec<u32>,
}

impl RegistryClient {
    pub fn add_node(&mut self, id: u32) {
        self.nodes.push(id);
    }
}

impl TryFrom<&DictRef> for RegistryClient {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        // I currently expect all these fields to be present for general usage
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let module_id = value
            .get(*MODULE_ID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("MODULE_ID"))?;
        let protocol = value
            .get(*PROTOCOL)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("PROTOCOL"))?;
        let process_id = value
            .get(*SEC_PID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_PID"))?;
        let user_id = value
            .get(*SEC_UID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_UID"))?;
        let group_id = value
            .get(*SEC_GID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("SEC_GID"))?;
        let access = value
            .get(*ACCESS)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("ACCESS"))?;
        let application_name = value
            .get(*APP_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("APP_NAME"))?;

        Ok(Self {
            object_serial,
            module_id,
            protocol,
            process_id,
            user_id,
            group_id,
            access,

            application_name,
            application_binary: None,

            _proxy: None,
            _listener: None,

            nodes: vec![],
        })
    }
}

#[allow(unused)]
pub(crate) struct RegistryClientNode {
    pub(crate) object_serial: u32,
    pub(crate) parent_id: u32,

    pub(crate) metadata: Option<Metadata>,

    pub(crate) application_name: String,
    pub(crate) node_name: String,

    pub(crate) volume: u8,
    pub(crate) media_title: Option<String>,

    pub(crate) n_input_ports: u8,
    pub(crate) n_output_ports: u8,
    pub(crate) is_running: Option<bool>,
    pub(crate) is_muted: bool,

    pub(crate) media_target: Option<Option<NodeTarget>>,

    pub(crate) proxy: Option<Node>,
    pub(crate) _listener: Option<NodeListener>,

    pub ports: EnumMap<Direction, HashMap<u32, RegistryPort>>,
}

impl TryFrom<&DictRef> for RegistryClientNode {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let parent_id = value
            .get(*CLIENT_ID)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("CLIENT_ID"))?;
        let application_name = value
            .get("application.name")
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("APPLICATION_NAME"))?;
        let node_name = value
            .get(*NODE_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("NODE_NAME"))?;

        Ok(Self {
            object_serial,
            parent_id,

            application_name,
            node_name,

            volume: 0,
            media_title: None,
            media_target: None,

            n_input_ports: 0,
            n_output_ports: 0,
            is_running: None,
            is_muted: false,

            proxy: None,
            _listener: None,

            metadata: None,
            ports: Default::default(),
        })
    }
}

impl RegistryClientNode {
    pub(crate) fn add_port(&mut self, id: u32, direction: Direction, port: RegistryPort) {
        self.ports[direction].insert(id, port);
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RegistryLink {
    pub(crate) object_serial: u32,

    pub input_node: u32,
    pub input_port: u32,
    pub output_node: u32,
    pub output_port: u32,
}

impl TryFrom<&DictRef> for RegistryLink {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let input_node = value
            .get(*LINK_INPUT_NODE)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_INPUT_NODE"))?;
        let input_port = value
            .get(*LINK_INPUT_PORT)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_INPUT_PORT"))?;
        let output_node = value
            .get(*LINK_OUTPUT_NODE)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_OUTPUT_NODE"))?;
        let output_port = value
            .get(*LINK_OUTPUT_PORT)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("LINK_OUTPUT_PORT"))?;

        Ok(RegistryLink {
            object_serial,
            input_node,
            input_port,
            output_node,
            output_port,
        })
    }
}

// pipewire-rs doesn't seem to provide one of these, it does have from_str and to_str, but they're
// crate public, so we can't use them, and they're only looking for the last chunk.
fn to_object_type(input: &str) -> ObjectType {
    match input {
        "PipeWire:Interface:Client" => ObjectType::Client,
        "PipeWire:Interface:ClientEndpoint" => ObjectType::ClientEndpoint,
        "PipeWire:Interface:ClientNode" => ObjectType::ClientNode,
        "PipeWire:Interface:ClientSession" => ObjectType::ClientSession,
        "PipeWire:Interface:Core" => ObjectType::Core,
        "PipeWire:Interface:Device" => ObjectType::Device,
        "PipeWire:Interface:Endpoint" => ObjectType::Endpoint,
        "PipeWire:Interface:EndpointLink" => ObjectType::EndpointLink,
        "PipeWire:Interface:EndpointStream" => ObjectType::EndpointStream,
        "PipeWire:Interface:Factory" => ObjectType::Factory,
        "PipeWire:Interface:Link" => ObjectType::Link,
        "PipeWire:Interface:Metadata" => ObjectType::Metadata,
        "PipeWire:Interface:Module" => ObjectType::Module,
        "PipeWire:Interface:Node" => ObjectType::Node,
        "PipeWire:Interface:Port" => ObjectType::Port,
        "PipeWire:Interface:Profiler" => ObjectType::Profiler,
        "PipeWire:Interface:Registry" => ObjectType::Registry,
        "PipeWire:Interface:Session" => ObjectType::Session,
        _ => ObjectType::Other(input.to_string()),
    }
}

impl Drop for PipewireRegistry {
    fn drop(&mut self) {
        debug!("Dropping Pipewire Registry");
    }
}
