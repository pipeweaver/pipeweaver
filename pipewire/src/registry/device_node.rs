use crate::Direction;
use crate::registry::port::RegistryPort;
use crate::store::Store;
use anyhow::{anyhow, bail};
use enum_map::EnumMap;
use log::debug;
use pipewire::core::Core;
use pipewire::keys::{
    DEVICE_ID, MEDIA_CLASS, NODE_DESCRIPTION, NODE_NAME, NODE_NICK, OBJECT_PATH, OBJECT_SERIAL,
};
use pipewire::node::{Node, NodeListener};
use pipewire::registry::{GlobalObject, Registry};
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
use std::rc::{Rc, Weak};

pub fn handle_device_node(
    id: u32,
    core: Rc<Core>,
    global: &GlobalObject<&DictRef>,
    registry: Rc<RefCell<Registry>>,
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

        let bound: Option<Node> = registry.borrow().bind(global).ok();
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
