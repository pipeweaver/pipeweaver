use crate::default_device::{DefaultDefinition, DefaultDevice};
use crate::manager::FilterData;
use crate::registry::{
    Direction, MetadataStore, RegistryClient, RegistryClientNode, RegistryDevice,
    RegistryDeviceNode, RegistryFactory, RegistryLink,
};
use crate::{
    ApplicationNode, DeviceNode, FilterProperty, FilterValue, LinkType, MediaClass, NodeTarget,
    PipewireReceiver,
};
use anyhow::Result;
use anyhow::{anyhow, bail};
use enum_map::{Enum, EnumMap};
use log::{debug, error, info, warn};
use oneshot::Sender;
use parking_lot::RwLock;
use pipewire::filter::{Filter, FilterListener, FilterPort};
use pipewire::link::{Link, LinkListener};
use pipewire::node::{Node, NodeListener, NodeState};
use pipewire::properties::Properties;
use pipewire::proxy::ProxyListener;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Pod, Property, Value, ValueArray, object};
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::mem::discriminant;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use ulid::Ulid;

pub(crate) enum TargetType {
    Node(Option<u32>),
    Serial(Option<u32>),
}

pub struct Store {
    // The main Session Proxy Metadata
    session_proxy: Option<MetadataStore>,
    settings_proxy: Option<MetadataStore>,

    // Pipewire Factories, helps us track types
    factories: HashMap<u32, RegistryFactory>,

    // The default Sink / Source
    default_sink: DefaultDevice,   // (eg pipeweaver_system)
    default_source: DefaultDevice, // (eg pipeweaver_chat_mic)

    // These are nodes, filters and links created by us
    managed_nodes: HashMap<Ulid, NodeStore>,
    managed_filters: HashMap<Ulid, FilterStore>,
    managed_links: HashMap<Ulid, LinkStore>,

    // These are devices and device nodes not created by us
    unmanaged_devices: HashMap<u32, RegistryDevice>,
    unmanaged_device_nodes: HashMap<u32, RegistryDeviceNode>,

    // These are clients and client nodes not created by us
    unmanaged_clients: HashMap<u32, RegistryClient>,
    unmanaged_client_nodes: HashMap<u32, RegistryClientNode>,

    // These are links found which aren't specifically between managed targets
    unmanaged_links: HashMap<u32, RegistryLink>,

    // Usable Nodes are unmanaged device / client nodes with a stereo setup
    usable_client_nodes: Vec<u32>,

    callback_tx: mpsc::Sender<PipewireReceiver>,
}

impl Store {
    pub fn new(callback_tx: mpsc::Sender<PipewireReceiver>) -> Self {
        Self {
            session_proxy: None,
            settings_proxy: None,

            factories: HashMap::new(),

            default_sink: DefaultDevice::default(),
            default_source: DefaultDevice::default(),

            managed_nodes: HashMap::new(),
            managed_filters: HashMap::new(),
            managed_links: HashMap::new(),

            unmanaged_devices: HashMap::new(),
            unmanaged_device_nodes: HashMap::new(),

            unmanaged_clients: HashMap::new(),
            unmanaged_client_nodes: HashMap::new(),

            unmanaged_links: HashMap::new(),

            usable_client_nodes: vec![],

            callback_tx,
        }
    }

    // Session Handler
    pub fn set_session_proxy(&mut self, session: MetadataStore) {
        if self.session_proxy.is_some() {
            warn!("Attempting to redefine default Session Manager, aborting.");
            return;
        }
        info!("Session Proxy Found");
        self.session_proxy = Some(session);
    }

    pub fn set_settings_proxy(&mut self, settings: MetadataStore) {
        if self.settings_proxy.is_some() {
            warn!("Attempting to redefine default Settings Manager, aborting.");
            return;
        }
        info!("Settings Proxy Found");
        self.settings_proxy = Some(settings);
    }

    pub fn announce_clock_rate(&self, rate: Option<u32>) {
        let _ = self
            .callback_tx
            .send(PipewireReceiver::AnnouncedClock(rate));
    }

    // ----- FACTORIES -----
    pub fn factory_add(&mut self, id: u32, factory: RegistryFactory) {
        self.factories.insert(id, factory);
    }

    pub fn _factory_get(&self, id: u32) -> Option<&RegistryFactory> {
        self.factories.get(&id)
    }

    // ----- SET DEFAULT DEVICES -----
    pub fn set_default_sink(&mut self, device: DefaultDefinition) {
        let changed = self.default_sink.set(device);

        if changed && self.find_default_sink_id() {
            debug!("Default Sink Updated to: {:?}", self.default_sink);
            self.send_default_sink();
        }
    }

    pub fn set_default_source(&mut self, device: DefaultDefinition) {
        let changed = self.default_source.set(device);

        if changed && self.find_default_source_id() {
            debug!("Default Source Updated to: {:?}", self.default_source);
            self.send_default_source();
        }
    }

    fn send_default_sink(&self) {
        self.send_default_update(&self.default_sink, MediaClass::Sink);
    }

    fn send_default_source(&self) {
        self.send_default_update(&self.default_source, MediaClass::Source);
    }

    fn send_default_update(&self, default: &DefaultDevice, class: MediaClass) {
        if let Some(node_id) = default.get_active_node_id() {
            let message = if self.is_managed_node(node_id) {
                let ulid = self.managed_node_find_by_node_id(node_id).unwrap();
                PipewireReceiver::DefaultChanged(class, NodeTarget::Node(ulid))
            } else {
                PipewireReceiver::DefaultChanged(class, NodeTarget::UnmanagedNode(node_id))
            };
            let _ = self.callback_tx.send(message);
        }
    }

    pub fn find_default_source_id(&mut self) -> bool {
        self.populate_default_node_ids(false)
    }

    pub fn find_default_sink_id(&mut self) -> bool {
        self.populate_default_node_ids(true)
    }

    fn populate_default_node_ids(&mut self, is_sink: bool) -> bool {
        // Grab the Device Names
        let device = match is_sink {
            true => &mut self.default_sink,
            false => &mut self.default_source,
        };
        let configured = device.get_configured().map(|s| s.to_string());
        let default = device.get_default().map(|s| s.to_string());

        // Try to find and set the configured device node
        let mut send_update = false;
        if let Some(configured) = configured
            && let Some(id) = self.find_node_by_name(&configured)
        {
            let device = match is_sink {
                true => &mut self.default_sink,
                false => &mut self.default_source,
            };
            if device.set_configured_node_id(id) {
                send_update = true;
            }
        }

        // Try to find and set the default device node
        if let Some(default) = default
            && let Some(id) = self.find_node_by_name(&default)
        {
            let device = match is_sink {
                true => &mut self.default_sink,
                false => &mut self.default_source,
            };
            if device.set_default_node_id(id) {
                send_update = true;
            }
        }
        send_update
    }

    fn find_node_by_name(&self, name: &str) -> Option<u32> {
        for node in self.managed_nodes.values() {
            if let Some(node_name) = node.props.get("node.name")
                && node_name == name
            {
                return node.pw_id;
            }
        }
        for (id, node) in &self.unmanaged_device_nodes {
            if let Some(node_name) = &node.name
                && node_name == name
            {
                return Some(*id);
            }
        }
        None
    }

    // ----- MANAGED NODES -----
    pub fn is_managed_node(&self, id: u32) -> bool {
        // Before we add this, is this a managed node?
        self.managed_nodes
            .values()
            .any(|node| node.pw_id == Some(id))
    }

    pub fn managed_node_add(&mut self, node: NodeStore) {
        debug!("[{}] Device Added to Store, waiting for data", &node.id);
        self.managed_nodes.insert(node.id, node);
    }

    pub fn managed_node_get(&self, id: Ulid) -> Option<&NodeStore> {
        self.managed_nodes.get(&id)
    }

    pub fn managed_node_remove(&mut self, id: Ulid) {
        // This should cause pipewire to drop the node as soon as it goes out of scope. We don't
        // check for things like links here, PW will clean them up, so upstream should manage
        // anything extra.
        if self.managed_nodes.contains_key(&id) {
            let node = self.managed_nodes.remove(&id);
            if let Some(node) = node
                && let Some(pw_id) = node.pw_id
            {
                if self.default_sink.device_removed(pw_id) {
                    self.send_default_sink();
                }
                if self.default_source.device_removed(pw_id) {
                    self.send_default_source();
                }
            }
        }
    }

    pub fn managed_node_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        let node_name = node.props.get("node.name").map(|s| s.to_string());
        node.pw_id.replace(pw_id);

        if let Some(name) = node_name {
            if self.default_sink.device_added(pw_id, &name) {
                self.send_default_sink();
            }
            if self.default_source.device_added(pw_id, &name) {
                self.send_default_source();
            }
        }

        self.managed_node_check_ready(id);
    }

    pub fn managed_node_set_pw_serial(&mut self, id: u32, serial: u32) {
        if let Some(owned) = self
            .managed_nodes
            .values_mut()
            .find(|v| v.pw_id.is_some_and(|e| e == id))
        {
            debug!("[{}] Pipewire Serial assigned: {}", owned.id, serial);
            owned.object_serial = Some(serial);
        }
    }

    pub fn managed_node_request_ports(&self, id: Ulid) {
        let node = self.managed_nodes.get(&id).expect("Broke");
        node.proxy
            .enum_params(0, Some(ParamType::PortConfig), 0, u32::MAX);
    }

    pub fn managed_node_add_port(&mut self, id: Ulid, location: PortLocation, port_id: u32) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        node.port_map[location] = Some(port_id);

        for location in PortLocation::iter() {
            if node.port_map[location].is_none() {
                return;
            }
        }

        // If we get here, all our ports have been set, trigger the ready event
        self.managed_node_ports_ready(id);
    }

    pub fn managed_node_ports_ready(&mut self, id: Ulid) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        node.ports_ready = true;
        self.managed_node_check_ready(id);
    }

    pub fn managed_node_check_ready(&mut self, id: Ulid) {
        let node = self
            .managed_nodes
            .get_mut(&id)
            .expect("Attempted to lookup non-existing node!");

        if node.ports_ready
            && node.pw_id.is_some()
            && let Some(sender) = node.ready_sender.take()
        {
            debug!("[{}] Device Ready, sending callback", &id);
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
        }
    }

    pub fn managed_node_find_by_node_id(&self, id: u32) -> Option<Ulid> {
        self.managed_nodes
            .iter()
            .find(|(_, node)| node.pw_id == Some(id))
            .map(|(id, _)| *id)
    }

    // ----- NODE VOLUMES -----
    pub fn set_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        let node = self
            .managed_nodes
            .get(&id)
            .ok_or(anyhow!("Failed to find node"))?;

        let volume = (volume as f32 / 100.0).powi(3);
        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume, volume]))),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod).unwrap();
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes) {
            node.proxy.set_param(ParamType::Props, 0, bytes);
        }
        Ok(())
    }

    pub fn set_application_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        let node = self
            .unmanaged_client_nodes
            .get(&id)
            .ok_or(anyhow!("Failed to find node"))?;

        let volume = (volume as f32 / 100.0).powi(3);
        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume, volume]))),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod).unwrap();
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes)
            && let Some(proxy) = &node.proxy
        {
            proxy.set_param(ParamType::Props, 0, bytes);
        }
        Ok(())
    }

    pub fn on_volume_change(&mut self, id: Ulid, volume: u8) {
        let _ = self
            .callback_tx
            .send(PipewireReceiver::NodeVolumeChanged(id, volume));
    }

    pub fn set_mute(&mut self, id: Ulid, muted: bool) -> Result<()> {
        let node = self
            .managed_nodes
            .get(&id)
            .ok_or(anyhow!("Failed to find node"))?;

        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_mute, Value::Bool(muted)),
        });
        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod)?;
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes) {
            node.proxy.set_param(ParamType::Props, 0, bytes);
        }
        Ok(())
    }
    pub fn set_application_muted(&mut self, id: u32, muted: bool) -> Result<()> {
        let node = self
            .unmanaged_client_node_get(id)
            .ok_or(anyhow!("Failed to find node"))?;
        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_mute, Value::Bool(muted)),
        });
        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod).unwrap();
        let bytes = cursor.into_inner();

        // Create the POD and send it to the proxy
        if let Some(bytes) = Pod::from_bytes(&bytes)
            && let Some(proxy) = &node.proxy
        {
            proxy.set_param(ParamType::Props, 0, bytes);
        }

        Ok(())
    }

    pub fn on_mute_change(&mut self, id: Ulid, muted: bool) {
        let _ = self
            .callback_tx
            .send(PipewireReceiver::NodeMuteChanged(id, muted));
    }

    // ----- MANAGED FILTERS -----
    pub fn managed_filter_add(&mut self, filter: FilterStore) {
        debug!("[{}] Filter Added to Store", &filter.id);
        self.managed_filters.insert(filter.id, filter);
    }

    pub fn managed_filter_get(&self, id: Ulid) -> Option<&FilterStore> {
        self.managed_filters.get(&id)
    }

    pub fn managed_filter_remove(&mut self, filter: Ulid) {
        self.managed_link_remove_for_type(LinkType::Filter(filter));
        self.managed_filters.remove(&filter);
    }

    pub fn managed_filter_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        filter.pw_id = Some(pw_id);

        if let Some(Some(sender)) = filter.ready_sender.take() {
            let _ = sender.send(());
        }
    }

    pub fn managed_filter_set_parameter(
        &mut self,
        id: Ulid,
        key: u32,
        value: FilterValue,
    ) -> Result<String> {
        // Find the filter
        let filter = self
            .managed_filters
            .get_mut(&id)
            .ok_or(anyhow!("Filter Not Found"))?;

        // Set the Property
        filter.data.write().callback.set_property(key, value)
    }

    pub fn managed_filter_get_parameters(&self, id: Ulid) -> Result<Vec<FilterProperty>> {
        // Find the filter
        let filter = self
            .managed_filters
            .get(&id)
            .ok_or(anyhow!("Filter Missing"))?;

        // Send the Properties
        Ok(filter.data.read().callback.get_properties())
    }

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

    pub fn managed_link_remove(&mut self, source: LinkType, destination: LinkType) {
        self.managed_links
            .retain(|_, link| link.source != source || link.destination != destination)
    }

    pub fn managed_link_remove_for_type(&mut self, id: LinkType) {
        self.managed_links
            .retain(|_, link| link.source != id && link.destination != id);
    }

    pub fn managed_link_ready(&mut self, id: Ulid, link_id: Ulid, pw_id: u32) {
        if let Some(link) = self.managed_links.get_mut(&id) {
            for port in PortLocation::iter() {
                if let Some(port) = &mut link.links[port]
                    && port.internal_id == link_id
                {
                    port.pw_id = Some(pw_id);

                    // This will be unmanaged before the link callback, so take ownership
                    self.unmanaged_links.remove(&pw_id);
                }
            }
        }

        // Regardless of what's happened here, perform a ready check on the parent
        self.managed_link_ready_check(id);
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

    // ----- UNMANAGED DEVICES -----
    pub fn unmanaged_device_add(&mut self, id: u32, device: RegistryDevice) {
        // Only add this if the node isn't already managed
        if self.is_managed_node(id) {
            return;
        }
        self.unmanaged_devices.insert(id, device);
    }

    pub fn unmanaged_device_get(&mut self, id: u32) -> Option<&mut RegistryDevice> {
        self.unmanaged_devices.get_mut(&id)
    }

    pub fn unmanaged_device_remove(&mut self, id: u32) {
        self.unmanaged_devices.remove(&id);
    }

    // ----- UNMANAGED DEVICE NODES -----
    pub fn unmanaged_device_node_add(&mut self, id: u32, node: RegistryDeviceNode) {
        debug!("Checking: {:?}", node);
        if self.is_managed_node(id) {
            return;
        }

        if let Some(name) = node.name.clone() {
            if self.default_sink.device_added(id, &name) {
                self.send_default_sink();
            }
            if self.default_source.device_added(id, &name) {
                self.send_default_source();
            }
        }

        self.unmanaged_device_nodes.insert(id, node);
        self.unmanaged_node_add(id);

        //self.unmanaged_node_update(id);
    }

    pub fn unmanaged_device_node_get(&mut self, id: u32) -> Option<&mut RegistryDeviceNode> {
        self.unmanaged_device_nodes.get_mut(&id)
    }

    pub fn unmanaged_device_node_remove(&mut self, id: u32) {
        // Need to flag upstream if the node has gone away
        if self.unmanaged_device_nodes.contains_key(&id) {
            let _ = self.callback_tx.send(PipewireReceiver::DeviceRemoved(id));
        }

        self.unmanaged_device_nodes.remove(&id);
        for client in self.unmanaged_devices.values_mut() {
            client.nodes.retain(|n| n != &id);
        }

        // Check to make sure these aren't defaults
        if self.default_sink.device_removed(id) {
            self.send_default_sink();
        }
        if self.default_source.device_removed(id) {
            self.send_default_source();
        }
    }

    /// When an unmanaged node comes in, we need to send it off across to the manager to be
    /// listed. I should probably move this into `unmanaged_device_node_add` :D
    pub fn unmanaged_node_add(&mut self, id: u32) {
        if let Some(node) = self.unmanaged_device_nodes.get(&id) {
            debug!("Node Arrived: {:?}", node);
            // We need a media class, otherwise we can't use this node
            if let Some(media_class) = &node.media_class {
                // Map the media class to our internal enum
                let media_class = match media_class {
                    s if s.starts_with("Audio/Sink") => Some(MediaClass::Sink),
                    s if s.starts_with("Audio/Source") => Some(MediaClass::Source),
                    s if s.starts_with("Audio/Duplex") => Some(MediaClass::Duplex),
                    _ => {
                        warn!("Unrecognized Media Class: {}", media_class);
                        None
                    }
                };

                if let Some(media_class) = media_class {
                    // Create the virtual node and send it upstream
                    let node = DeviceNode {
                        node_id: id,
                        node_class: media_class,
                        is_usable: false,
                        name: node.name.clone(),
                        nickname: node.nickname.clone(),
                        description: node.description.clone(),
                    };

                    let _ = self.callback_tx.send(PipewireReceiver::DeviceAdded(node));
                }
            }
        }
    }

    /// This is called whenever the port status changes, we need to check whether this is a
    /// regular stereo node or not, so we can report it as usable.
    pub fn unmanaged_node_update(&mut self, id: u32) {
        let is_usable = self.is_usable_unmanaged_device_node(id).is_some();

        if let Some(node) = self.unmanaged_device_nodes.get_mut(&id)
            && node.is_usable != is_usable
        {
            node.is_usable = is_usable;
            let message = PipewireReceiver::DeviceUsable(id, is_usable);
            let _ = self.callback_tx.send(message);
        }
    }

    pub fn is_usable_unmanaged_device_node(&self, id: u32) -> Option<MediaClass> {
        if let Some(node) = self.unmanaged_device_nodes.get(&id) {
            // If we don't have a name or description, we can't use this node
            if node.name.is_none() && node.description.is_none() {
                return None;
            }

            let mut in_count = 0;
            let mut out_count = 0;

            for (direction, ports) in &node.ports {
                let count = ports.values().filter(|port| !port.is_monitor).count();

                match direction {
                    Direction::In => in_count += count,
                    Direction::Out => out_count += count,
                }
            }

            // Return the Specific MediaClass based on Channel Count
            return self.get_media_class(in_count, out_count);
        }
        None
    }

    // ----- UNMANAGED CLIENT -----
    pub fn unmanaged_client_add(&mut self, id: u32, device: RegistryClient) {
        // Only add this if the node isn't already managed
        self.unmanaged_clients.insert(id, device);
    }

    pub fn unmanaged_client_set_binary(&mut self, id: u32, name: String) {
        let nodes = if let Some(client) = self.unmanaged_clients.get_mut(&id) {
            client.application_binary = Some(name);
            client.nodes.clone()
        } else {
            vec![]
        };

        // Check all the client nodes to see if they were waiting for this
        for node in nodes {
            self.unmanaged_client_node_check(node);
        }
    }

    pub fn unmanaged_client_get(&mut self, id: u32) -> Option<&mut RegistryClient> {
        self.unmanaged_clients.get_mut(&id)
    }

    pub fn unmanaged_client_remove(&mut self, id: u32) {
        self.unmanaged_clients.remove(&id);
    }

    // ----- UNMANAGED CLIENT NODES -----
    pub fn unmanaged_client_node_add(&mut self, id: u32, node: RegistryClientNode) {
        self.unmanaged_client_nodes.insert(id, node);
    }

    pub fn unmanaged_client_node_get(&mut self, id: u32) -> Option<&mut RegistryClientNode> {
        self.unmanaged_client_nodes.get_mut(&id)
    }

    pub fn unmanaged_client_node_set_volume(&mut self, id: u32, volume: u8) {
        if let Some(node) = self.unmanaged_client_node_get(id)
            && node.volume != volume
        {
            node.volume = volume;

            let _ = self
                .callback_tx
                .send(PipewireReceiver::ApplicationVolumeChanged(id, volume));
        }
    }

    // Naming here is terrible
    pub fn unmanaged_client_node_set_mute(&mut self, id: u32, muted: bool) {
        if let Some(node) = self.unmanaged_client_node_get(id) {
            if node.is_muted != muted {
                node.is_muted = muted;
                let _ = self
                    .callback_tx
                    .send(PipewireReceiver::ApplicationMuteChanged(id, muted));
            }
        } else {
            error!("Failed to locate Application Node");
        }
    }

    pub fn unmanaged_client_node_set_media(&mut self, id: u32, media: String) {
        if let Some(node) = self.unmanaged_client_node_get(id) {
            if node.media_title.is_none() && media == "AudioStream" {
                // TODO: A better job of this :p
                // Do nothing, already setup?
            } else if node.media_title.is_some() && media == "AudioStream" {
                node.media_title = None;
            } else if node.media_title != Some(media.clone()) {
                node.media_title = Some(media.clone());
                let _ = self
                    .callback_tx
                    .send(PipewireReceiver::ApplicationTitleChanged(id, media));
            }
        }
    }

    pub fn unmanaged_client_node_set_target(&mut self, id: u32, target: TargetType) {
        // So we need to locate the target, which might be tricky as the target is passed as an
        // object serial, and not a node id, meaning we need to do some digging.

        let mut result: Option<NodeTarget> = None;

        match target {
            TargetType::Node(Some(id)) => {
                for node in self.managed_nodes.values() {
                    if let Some(object_id) = node.pw_id
                        && object_id == id
                    {
                        result = Some(NodeTarget::Node(node.id));
                        break;
                    }
                }

                // If we get here, it's not a managed node, so check and send as unmanaged
                if self.unmanaged_device_nodes.contains_key(&id) {
                    result = Some(NodeTarget::UnmanagedNode(id));
                }

                if result.is_none() {
                    debug!("Node not found: {}", id);
                }
            }
            TargetType::Serial(Some(id)) => {
                for node in self.managed_nodes.values() {
                    if let Some(object_serial) = node.object_serial
                        && object_serial == id
                    {
                        result = Some(NodeTarget::Node(node.id));
                        break;
                    }
                }

                // Can't find it, we need to look for object serials in the unmanaged list
                for (node_id, node) in &self.unmanaged_device_nodes {
                    if node.object_serial == id {
                        result = Some(NodeTarget::UnmanagedNode(*node_id));
                        break;
                    }
                }
            }
            _ => {
                warn!("Blank TargetType Received!");
            }
        }

        if let Some(client) = self.unmanaged_client_node_get(id) {
            client.media_target = Some(result);

            if self.usable_client_nodes.contains(&id) {
                // We're already defined, send the node update
                let _ = self
                    .callback_tx
                    .send(PipewireReceiver::ApplicationTargetChanged(id, result));
            } else {
                // Check whether we're ready to send
                self.unmanaged_client_node_check(id);
            }
        } else {
            debug!("Route for {} is not Managed", id);
        }
    }

    pub fn unmanaged_client_node_set_state(&mut self, id: u32, state: NodeState) {
        let is_running = discriminant(&state) == discriminant(&NodeState::Running);

        if let Some(node) = self.unmanaged_client_node_get(id) {
            match node.is_running {
                None => {
                    node.is_running = Some(is_running);
                    self.unmanaged_client_node_check(id);
                }
                Some(node_state) => {
                    if node_state == is_running {
                        return;
                    }

                    node.is_running = Some(is_running);
                    if !is_running {
                        // We've gone from Running -> Not Running, flag the client as removed
                        self.unmanaged_client_clear_usable(id);
                    } else {
                        // We've moved into a Running state, so perform a check.
                        self.unmanaged_client_node_check(id);
                    }
                }
            }
        }
    }

    pub fn unmanaged_client_node_remove(&mut self, id: u32) {
        // Need to flag upstream if the node has gone away
        if self.usable_client_nodes.contains(&id) {
            let _ = self
                .callback_tx
                .send(PipewireReceiver::ApplicationRemoved(id));
            self.usable_client_nodes.retain(|v| v != &id);
        }

        self.unmanaged_client_nodes.remove(&id);
        for client in self.unmanaged_clients.values_mut() {
            client.nodes.retain(|n| n != &id);
        }
    }

    pub fn unmanaged_client_clear_usable(&mut self, id: u32) {
        let message = PipewireReceiver::ApplicationRemoved(id);
        let _ = self.callback_tx.send(message);

        // Remove the usable node, we can re-establish it later
        self.usable_client_nodes.retain(|v| v != &id);
    }

    pub fn unmanaged_client_node_check(&mut self, id: u32) {
        if self.usable_client_nodes.contains(&id) {
            // We already know this is usable, so don't trigger again
            return;
        }

        if let Some(node) = self.unmanaged_client_nodes.get(&id)
            && let Some(media_type) = self.is_usable_unmanaged_client_node(id)
            && let Some(parent) = self.unmanaged_clients.get(&node.parent_id)
            && !self.usable_client_nodes.contains(&id)
        {
            self.usable_client_nodes.push(id);
            let node = ApplicationNode {
                node_id: id,
                node_class: media_type,
                media_target: node.media_target,

                volume: node.volume,
                muted: node.is_muted,

                title: node.media_title.clone(),

                name: node.application_name.clone(),

                // We can safely panic! here, is_usable_unamanged_client_node checks this.
                process_name: parent.application_binary.clone().expect("NO BINARY"),
            };

            let message = PipewireReceiver::ApplicationAdded(node);
            let _ = self.callback_tx.send(message);
        }
    }

    pub fn is_usable_unmanaged_client_node(&self, id: u32) -> Option<MediaClass> {
        if let Some(node) = self.unmanaged_client_nodes.get(&id) {
            if node.node_name.is_empty()
                || node.application_name.is_empty()
                || node.is_running.is_none()
                || node.is_running == Some(false)
            {
                return None;
            }

            // We need the parent to have an application binary
            if let Some(parent) = self.unmanaged_clients.get(&id) {
                parent.application_binary.as_ref()?;
            }

            let mut in_count = 0;
            let mut out_count = 0;
            for (direction, ports) in &node.ports {
                let count = ports.values().filter(|port| !port.is_monitor).count();

                match direction {
                    Direction::In => in_count += count,
                    Direction::Out => out_count += count,
                }
            }

            // Return the Specific MediaClass based on Channel Count
            return self.get_media_class(in_count, out_count);
        }

        None
    }

    pub fn unmanaged_node_set_meta(
        &mut self,
        id: u32,
        key: String,
        type_: Option<String>,
        value: Option<String>,
    ) {
        if let Some(session) = &self.session_proxy {
            session
                .metadata
                .set_property(id, &key, type_.as_deref(), value.as_deref())
        }
    }

    // ----- UNMANAGED LINKS -----
    pub fn unmanaged_link_add(&mut self, id: u32, link: RegistryLink) {
        // Check our Managed Links to see if this is actually unmanaged
        if self.is_managed_link(id).is_none() {
            self.unmanaged_links.insert(id, link);
        }
    }

    pub fn unmanaged_link_remove(&mut self, id: u32) {
        self.unmanaged_links.remove(&id);
    }

    pub fn get_unmanaged_links(&self) -> &HashMap<u32, RegistryLink> {
        &self.unmanaged_links
    }

    // ----- REMOVE HANDLER -----
    // PipeWire doesn't inform us of the type which is being removed, just the ID, so we need
    // to go through our stored data, find the corresponding item, and handle it.
    pub fn remove_by_id(&mut self, id: u32) {
        if self.unmanaged_devices.contains_key(&id) {
            return self.unmanaged_device_remove(id);
        }

        if self.unmanaged_device_nodes.contains_key(&id) {
            return self.unmanaged_device_node_remove(id);
        }

        if self.unmanaged_clients.contains_key(&id) {
            return self.unmanaged_client_remove(id);
        }

        if self.unmanaged_client_nodes.contains_key(&id) {
            return self.unmanaged_client_node_remove(id);
        }

        if self.unmanaged_links.contains_key(&id) {
            return self.unmanaged_link_remove(id);
        }

        // Something may be trying to mess with a managed link, if so, completely drop our links
        // and report back to whatever is calling us that it's happened, so they can action it.
        if let Some(id) = self.is_managed_link(id)
            && let Some(link) = self.managed_links.remove(&id)
        {
            let _ = self.callback_tx.send(PipewireReceiver::ManagedLinkDropped(
                link.source,
                link.destination,
            ));
        }
    }

    // ----- UTILITY FUNCTIONS -----
    fn get_media_class(&self, in_count: usize, out_count: usize) -> Option<MediaClass> {
        // Return the Specific MediaClass based on Channel Count
        if in_count >= 1 && out_count == 0 {
            return Some(MediaClass::Sink);
        } else if out_count >= 1 && in_count == 0 {
            return Some(MediaClass::Source);
        } else if in_count >= 1 && in_count == out_count {
            // This is a bit of an assumption really, but we have non-monitor ports on the
            // tail end, so a reasonable assumption.
            return Some(MediaClass::Duplex);
        }
        None
    }
}

pub(crate) struct NodeStore {
    pub(crate) pw_id: Option<u32>,
    pub(crate) object_serial: Option<u32>,

    pub(crate) id: Ulid,
    pub(crate) props: Properties,

    pub(crate) proxy: Node,
    pub(crate) _proxy_listener: ProxyListener,
    pub(crate) _listener: NodeListener,

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

    /// The Ulid Identifier
    pub(crate) id: Ulid,

    // This maintains a general port map of location -> index
    pub(crate) port_map: EnumMap<Direction, EnumMap<PortLocation, u32>>,

    /// Details of the ports assigned to this filter
    pub(crate) _input_ports: Rc<RefCell<Vec<FilterPort>>>,
    pub(crate) _output_ports: Rc<RefCell<Vec<FilterPort>>>,

    /// These two fields need to exist purely to prevent the filter and the listener from
    /// being dropped, they're never directly accessed, they're just a store.
    pub(crate) _filter: Filter,

    /// The 'Ready Sender' is called once the filter is setup and ready-to-go
    pub(crate) ready_sender: Option<Option<Sender<()>>>,

    /// The Data related to the filter, including the sample processing callback
    pub data: Rc<RwLock<FilterData>>,
}

pub struct LinkStore {
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
    pub(crate) _link: Link,
    pub(crate) _listener: LinkListener,

    /// Internal Port Index Mapping
    pub(crate) _source_port_id: u32,
    pub(crate) _destination_port_id: u32,
}

#[derive(Debug, Enum, EnumIter, Copy, Clone, PartialEq)]
pub(crate) enum PortLocation {
    Left,
    Right,
}

impl Display for PortLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PortLocation::Left => write!(f, "FL"),
            PortLocation::Right => write!(f, "FR"),
        }
    }
}

impl FromStr for PortLocation {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FL" | "AUX_0" => Ok(Self::Left),
            "FR" | "AUX_1" => Ok(Self::Right),
            _ => bail!("Unknown Channel"),
        }
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        debug!("Dropping Pipewire Store");
    }
}
