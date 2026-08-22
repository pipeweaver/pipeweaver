use crate::registry::port::RegistryPort;
use crate::store::Store;
use crate::store::utils::get_media_class;
use crate::{DeviceNode, Direction, MediaClass, NodePort};
use anyhow::{anyhow, bail};
use enum_map::EnumMap;
use log::debug;
use pipewire::core::CoreRc;
use pipewire::keys::{
    DEVICE_ID, MEDIA_CLASS, NODE_DESCRIPTION, NODE_NAME, NODE_NICK, OBJECT_PATH, OBJECT_SERIAL,
};
use pipewire::node::{Node, NodeListener};
use pipewire::registry::{GlobalObject, RegistryRc};
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Pod, Property, Value, ValueArray, object};
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils;
use pipewire::spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::rc::Weak;
use strum::IntoEnumIterator;

pub fn handle_device_node(
    id: u32,
    core: CoreRc,
    global: &GlobalObject<&DictRef>,
    registry: RegistryRc,
    store: &mut Store,
    listener_store: Weak<RefCell<Store>>,
) {
    if let Some(props) = global.props
        && let Ok(mut node) = RegistryDeviceNode::try_from(props)
    {
        if let Some(parent_id) = node.parent_id
            && let Some(device) = store.unmanaged_device_get(parent_id)
        {
            device.add_node(id);
        }

        let bound: Option<Node> = registry.bind(global).ok();
        let info_local = listener_store.clone();
        let core_local = core.clone();
        if let Some(proxy) = bound {
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let inputs = info.n_input_ports();
                    let outputs = info.n_output_ports();

                    if let Some(store) = info_local.upgrade() {
                        let mut store = store.borrow_mut();

                        if store.unmanaged_device_node_get(id).is_some() {
                            store.unmanaged_node_port_count_update(id, inputs, outputs);

                            if info.props().is_some() && store.unmanaged_node_set_clock_ready(id) {
                                let seq = core_local.sync(0).expect("core sync failed");
                                store.add_pending_device_sync(seq.raw(), id);
                            }
                        }
                    }
                })
                .register();

            node._proxy = Some(proxy);
            node._listener = Some(listener);
        }
        // All unmanaged nodes should be handled, even if they don't have a parent
        store.unmanaged_device_node_add(id, node);
    }
}

pub(crate) struct RegistryDeviceNode {
    pub object_serial: u32,
    pub parent_id: Option<u32>,
    pub object_path: Option<String>,

    pub media_class: Option<String>,
    pub is_usable: bool,
    pub clock_ready: bool,
    pub is_synced: bool,

    pub volume: u8,
    pub muted: bool,

    pub nickname: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,

    pub(crate) _proxy: Option<Node>,
    pub(crate) _listener: Option<NodeListener>,

    pub port_count: EnumMap<Direction, Option<u32>>,
    pub ports: EnumMap<Direction, HashMap<u32, RegistryPort>>,

    /// Tracks whether this device has been sent upstream via DeviceAdded
    pub sent_upstream: bool,
}

impl TryFrom<&DictRef> for RegistryDeviceNode {
    type Error = anyhow::Error;

    fn try_from(value: &DictRef) -> Result<Self, Self::Error> {
        let object_serial = value
            .get(*OBJECT_SERIAL)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| anyhow!("OBJECT_SERIAL"))?;
        let parent_id = value.get(*DEVICE_ID).and_then(|s| s.parse::<u32>().ok());
        let object_path = value.get(*OBJECT_PATH).map(|s| s.to_string());
        let nickname = value.get(*NODE_NICK).map(|s| s.to_string());
        let description = value.get(*NODE_DESCRIPTION).map(|s| s.to_string());
        let name = value.get(*NODE_NAME).map(|s| s.to_string());
        let media_class = value.get(*MEDIA_CLASS).map(|s| s.to_string());

        // We need to match the media type here, it's only a device if it's a Sink or Source
        if let Some(media_class) = &media_class {
            if !media_class.starts_with("Audio/Source") && !media_class.starts_with("Audio/Sink") {
                bail!("Not an Audio Device Node");
            }
            if media_class.ends_with("/Internal") {
                bail!("Internal Device Node");
            }
        } else {
            bail!("Missing Media Class");
        }

        Ok(Self {
            object_serial,
            parent_id,
            object_path,

            media_class,
            is_usable: false,
            clock_ready: false,
            is_synced: false,

            volume: 0,
            muted: false,

            nickname,
            description,
            name,

            _proxy: None,
            _listener: None,

            port_count: EnumMap::default(),
            ports: Default::default(),
            sent_upstream: false,
        })
    }
}

impl Debug for RegistryDeviceNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryDeviceNode")
            .field("object_serial", &self.object_serial)
            .field("parent_id", &self.parent_id)
            .field("media_class", &self.media_class)
            .field("is_usable", &self.is_usable)
            .field("nickname", &self.nickname)
            .field("description", &self.description)
            .field("name", &self.name)
            .finish()
    }
}

impl RegistryDeviceNode {
    pub(crate) fn add_port(&mut self, direction: Direction, port: RegistryPort) {
        self.ports[direction].insert(port.global_id, port);
    }

    pub fn profile_port(&self) -> Option<u32> {
        let path = self.object_path.as_deref()?;
        let mut parts = path.split(':');
        parts.nth(3)?.parse().ok()
    }

    pub(crate) fn ports_desynced(&self) -> bool {
        for direction in Direction::iter() {
            let Some(expected) = self.port_count[direction] else {
                return true;
            };
            if self.ports[direction].len() as u32 != expected {
                return true;
            }
        }
        false
    }

    pub(crate) fn usable_media_class(&self) -> Option<MediaClass> {
        // If we don't have a name or description, we can't use this node
        if self.name.is_none() && self.description.is_none() {
            return None;
        }

        let mut in_count = 0;
        let mut out_count = 0;

        for (direction, ports) in &self.ports {
            let non_monitor: Vec<_> = ports.values().filter(|p| !p.is_monitor).collect();
            let count = if non_monitor.len() > 2 {
                // We should consider things like 5.1 devices valid, so long as there's a FL / FR
                let has_left = non_monitor
                    .iter()
                    .any(|p| p.channel == "FL" || p.channel == "AUX0");
                let has_right = non_monitor
                    .iter()
                    .any(|p| p.channel == "FR" || p.channel == "AUX1");

                // If we have them, force this count to 2, which will pass classify_media_class
                if has_left && has_right {
                    2
                } else {
                    non_monitor.len()
                }
            } else {
                non_monitor.len()
            };

            match direction {
                Direction::In => in_count += count,
                Direction::Out => out_count += count,
            }
        }

        get_media_class(in_count, out_count)
    }

    /// Build the public `DeviceNode` representation of this node, ready to send upstream.
    pub(crate) fn to_device_node(
        &self,
        id: u32,
        node_class: MediaClass,
        is_usable: bool,
    ) -> DeviceNode {
        let mut ports: EnumMap<Direction, Vec<NodePort>> = Default::default();
        for direction in Direction::iter() {
            for port in self.ports[direction].values() {
                // Don't send Monitor ports
                if !port.is_monitor {
                    ports[direction].push(NodePort {
                        name: port.name.clone(),
                        channel: port.channel.clone(),
                    });
                }
            }
        }

        DeviceNode {
            node_id: id,
            node_class,
            is_usable,
            name: self.name.clone(),
            nickname: self.nickname.clone(),
            description: self.description.clone(),

            volume: self.volume,
            muted: self.muted,

            ports,
        }
    }

    pub fn set_volume(&self, volume: u8) {
        let Some(proxy) = &self._proxy else {
            debug!("Proxy not active for node");
            return;
        };

        let volume = (volume as f32 / 100.0).powi(3);
        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume, volume]))),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod).unwrap();
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes) {
            proxy.set_param(ParamType::Props, 0, bytes);
        }
    }

    pub fn set_mute(&self, muted: bool) {
        let Some(proxy) = &self._proxy else {
            debug!("Proxy not active for node");
            return;
        };

        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_mute, Value::Bool(muted)),
        });
        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod).unwrap();
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes) {
            proxy.set_param(ParamType::Props, 0, bytes);
        }
    }
}
