use crate::registry::port::RegistryPort;
use crate::store::Store;
use crate::{Direction, NodeTarget};
use anyhow::anyhow;
use enum_map::EnumMap;
use pipewire::keys::{CLIENT_ID, MEDIA_CLASS, MEDIA_NAME, NODE_NAME, OBJECT_SERIAL};
use pipewire::metadata::Metadata;
use pipewire::node::{Node, NodeChangeMask, NodeListener};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::Value::Bool;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{Value, ValueArray};
use pipewire::spa::sys::{SPA_PARAM_Props, SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::{Rc, Weak};

pub fn handle_client_node(
    id: u32,
    global: &GlobalObject<&DictRef>,
    registry: Rc<RefCell<Registry>>,
    store: &mut Store,
    listener_store: Weak<RefCell<Store>>,
) {
    if let Some(props) = global.props
        && let Ok(mut node) = RegistryClientNode::try_from(props)
        && let Some(client) = store.unmanaged_client_get(node.parent_id)
    {
        let bound: Option<Node> = registry.borrow().bind(global).ok();
        if let Some(proxy) = bound {
            let param_local = listener_store.clone();
            let info_local = listener_store.clone();
            let listener = proxy
                .add_listener_local()
                .param(move |_seq, _type, _index, _next, param| {
                    if let Some(pod) = param {
                        let pod =
                            PodDeserializer::deserialize_any_from(pod.as_bytes()).map(|(_, v)| v);

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
                            if let Some(prop) = prop
                                && let Bool(value) = prop.value
                                && let Some(param_local) = param_local.upgrade()
                            {
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
                            && let Some(info_local) = info_local.upgrade()
                        {
                            info_local
                                .borrow_mut()
                                .unmanaged_client_node_set_media(id, String::from(media));
                        }

                        if change == NodeChangeMask::STATE
                            && let Some(info_local) = info_local.upgrade()
                        {
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

impl Debug for RegistryClientNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryClientNode")
            .field("object_serial", &self.object_serial)
            .field("parent_id", &self.parent_id)
            .field("metadata", &self.metadata)
            .field("application_name", &self.application_name)
            .field("node_name", &self.node_name)
            .field("volume", &self.volume)
            .field("media_title", &self.media_title)
            .field("n_input_ports", &self.n_input_ports)
            .field("n_output_ports", &self.n_output_ports)
            .field("is_running", &self.is_running)
            .field("is_muted", &self.is_muted)
            .field("media_target", &self.media_target)
            .field("ports", &self.ports)
            .finish()
    }
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
            .or_else(|| value.get("media.name"))
            .or_else(|| value.get(*NODE_NAME))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("APPLICATION_NAME"))?;
        let node_name = value
            .get(*NODE_NAME)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("NODE_NAME"))?;

        // If we don't have a stream media class, we're not an audio stream.
        value
            .get(*MEDIA_CLASS)
            .filter(|c| c.starts_with("Stream/"))
            .ok_or_else(|| anyhow!("Not a stream node"))?;

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
