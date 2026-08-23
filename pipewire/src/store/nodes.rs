//! Lifecycle of the nodes *we* create (as opposed to `unmanaged_devices`,
//! which tracks nodes pipewire owns): registration, waiting for pipewire to
//! assign an id and ports, state tracking, and volume/mute control.

use super::{ManagedNode, NodeStoreState, PortLocation, Store};
use crate::{MediaClass, NodeProperties, PipewireReceiver};
use anyhow::{Result, anyhow};
use log::{debug, error};
use pipewire::core::CoreRc;
use pipewire::keys::{
    APP_ICON_NAME, AUDIO_CHANNELS, DEVICE_ICON_NAME, FACTORY_NAME, MEDIA_CLASS, MEDIA_ICON_NAME,
    NODE_ALWAYS_PROCESS, NODE_DESCRIPTION, NODE_DRIVER, NODE_FORCE_QUANTUM, NODE_FORCE_RATE,
    NODE_GROUP, NODE_NAME, NODE_NICK, NODE_VIRTUAL, OBJECT_LINGER, PORT_MONITOR,
};
use pipewire::node::NodeChangeMask;
use pipewire::properties::properties;
use pipewire::proxy::ProxyT;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{Value, ValueArray};
use pipewire::spa::sys::{
    SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR, SPA_FORMAT_AUDIO_position,
    SPA_PARAM_PORT_CONFIG_format, SPA_PARAM_PortConfig, SPA_PARAM_Props, SPA_PROP_channelVolumes,
    SPA_PROP_mute,
};
use std::cell::RefCell;
use std::rc::Weak;
use ulid::Ulid;

impl Store {
    // ----- MANAGED NODES -----
    pub fn is_managed_node(&self, id: u32) -> bool {
        // Before we add this, is this a managed node?
        self.managed_nodes
            .values()
            .any(|node| node.pw_id == Some(id))
    }

    pub fn create_node(
        &mut self,
        core: &CoreRc,
        properties: NodeProperties,
        listener_store: Weak<RefCell<Store>>,
    ) -> Result<()> {
        let node_properties = &mut properties! {
            *FACTORY_NAME => "support.null-audio-sink",
            *NODE_NAME => properties.node_name.clone(),
            *NODE_NICK => properties.node_nick,
            *NODE_DESCRIPTION => properties.node_description,

            *NODE_ALWAYS_PROCESS => "true",
            *NODE_VIRTUAL => "true",
            *PORT_MONITOR => "false",

            *APP_ICON_NAME => &*properties.app_id,
            *MEDIA_ICON_NAME => &*properties.app_id,
            *DEVICE_ICON_NAME => &*properties.app_id,

            *NODE_GROUP => "pipeweaver-nodes",

            //*APP_NAME => properties.app_name,
            *OBJECT_LINGER => match properties.linger {
                true => "true",
                false => "false"
            },
            *MEDIA_CLASS => match properties.class {
                MediaClass::Source => "Audio/Source/Virtual",
                MediaClass::Duplex => "Audio/Duplex",
                MediaClass::Sink => "Audio/Sink",
            },

            *AUDIO_CHANNELS => "2",

            // Force the RATE to match the system rate
            *NODE_FORCE_RATE => properties.rate.to_string(),

            // We don't want to set a driver here. If creating a large number of nodes each of them
            // will pick a different device while finding a clock source, resulting in the nodes
            // being spread all over the place. When the node tree starts getting linked together
            // pipewire needs to pull all the nodes / audio_filters / devices into a single clock source
            // which can cause some pretty aggressive behaviours (I've seen it infinite loop as
            // various nodes fight for clock control).
            //
            // Setting this to false means that the devices will fall under the 'Dummy' node until
            // a physical device is attached, at which point it'll move everything together under
            // that single clock.
            *NODE_DRIVER => "false",

            // https://gitlab.freedesktop.org/pipewire/pipewire/-/wikis/Virtual-Devices
            "audio.position" => "FL,FR",

            // If upstream is managing the volumes via a filter, we don't want Pipewire interfering
            "monitor.channel-volumes" => match properties.managed_volume {
                true => "false",
                false => "true"
            },
        };

        // If a quantum is provided, send it in to the props
        if let Some(quantum) = properties.buffer {
            node_properties.insert(*NODE_FORCE_QUANTUM, quantum.to_string());
        }

        debug!(
            "[{}] Attempting to Create Device '{}'",
            properties.node_id, properties.node_name
        );

        // Properties built, create the node.
        let proxy = core
            .create_object::<pipewire::node::Node>("adapter", node_properties)
            .map_err(|e| anyhow!("Unable to Create Node {}", e))?;

        // Set the Initial volume
        super::utils::send_volume(&proxy, properties.initial_volume);

        debug!("[{}] Registering Proxy Listener", properties.node_id);
        let proxy_id = properties.node_id;
        let proxy_store = listener_store.clone();
        let proxy_listener = proxy
            .upcast_ref()
            .add_listener_local()
            .bound(move |id| {
                debug!("[{}] Pipewire NodeID assigned: {}", proxy_id, id);
                if let Some(proxy_store) = proxy_store.upgrade() {
                    proxy_store
                        .borrow_mut()
                        .managed_node_set_pw_id(proxy_id, id);
                }
            })
            .removed(|| {
                debug!("Removed..");
            })
            .register();

        debug!("[{}] Registering Node Listener", properties.node_id);
        let listener_id = properties.node_id;
        let listener_info_store = listener_store.clone();
        let listener_param_store = listener_store.clone();
        let listener = proxy
            .add_listener_local()
            .info(move |info| {
                // Check whether this is a PORT related message
                if info.change_mask().contains(NodeChangeMask::INPUT_PORTS)
                    || info.change_mask().contains(NodeChangeMask::OUTPUT_PORTS)
                {
                    // Now check whether our port count matches what's expected
                    if info.n_input_ports() == 2 && info.n_output_ports() == 2 {
                        debug!(
                            "[{}] Ports have appeared, requesting configuration",
                            listener_id
                        );
                        if let Some(store) = listener_info_store.upgrade() {
                            store.borrow().managed_node_request_ports(listener_id);
                        }
                    }
                }

                if info.change_mask().contains(NodeChangeMask::STATE) {
                    let new_state = NodeStoreState::from(info.state());

                    if let Some(store) = listener_info_store.upgrade() {
                        store
                            .borrow_mut()
                            .managed_node_state_changed(listener_id, new_state);
                    }
                }
            })
            .param(move |_seq, _type, _index, _next, param| {
                if let Some(pod) = param {
                    let pod = PodDeserializer::deserialize_any_from(pod.as_bytes()).map(|(_, v)| v);
                    if let Ok(Value::Object(object)) = pod {
                        if object.id == SPA_PARAM_PortConfig {
                            debug!("[{}] Port configuration Received", listener_id);
                            let prop = object
                                .properties
                                .iter()
                                .find(|p| p.key == SPA_PARAM_PORT_CONFIG_format);

                            // Format is optional
                            if let Some(prop) = prop
                                && let Value::Object(object) = &prop.value
                            {
                                // Value is of type SPA_TYPE_OBJECT_Format
                                let prop = object
                                    .properties
                                    .iter()
                                    .find(|p| p.key == SPA_FORMAT_AUDIO_position);

                                if let Some(prop) = prop {
                                    // Fucking hell, I hate how deep this is getting
                                    if let Value::ValueArray(ValueArray::Id(array)) = &prop.value
                                        && let Some(listener_param_store) =
                                            listener_param_store.upgrade()
                                    {
                                        let mut store = listener_param_store.borrow_mut();
                                        for (index, value) in array.iter().enumerate() {
                                            let index = index as u32;
                                            if value.0 == SPA_AUDIO_CHANNEL_FL {
                                                store.managed_node_add_port(
                                                    listener_id,
                                                    PortLocation::Left,
                                                    index,
                                                );
                                            }
                                            if value.0 == SPA_AUDIO_CHANNEL_FR {
                                                store.managed_node_add_port(
                                                    listener_id,
                                                    PortLocation::Right,
                                                    index,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        } else if object.id == SPA_PARAM_Props {
                            let prop = object
                                .properties
                                .iter()
                                .find(|p| p.key == SPA_PROP_channelVolumes);

                            // Get the Left / Right value
                            if let Some(prop) = prop
                                && let Value::ValueArray(ValueArray::Float(value)) = &prop.value
                            {
                                // OK, so KDE and pwvucontrol use the highest value for their reference
                                let max = value
                                    .iter()
                                    .copied()
                                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                                    .unwrap();

                                let volume = (max.cbrt() * 100.0).round() as u8;
                                if let Some(listener_param_store) = listener_param_store.upgrade() {
                                    listener_param_store
                                        .borrow_mut()
                                        .on_volume_change(listener_id, volume);
                                }
                            }

                            let prop = object.properties.iter().find(|p| p.key == SPA_PROP_mute);

                            if let Some(prop) = prop
                                && let Value::Bool(enabled) = &prop.value
                                && let Some(listener_param_store) = listener_param_store.upgrade()
                            {
                                listener_param_store
                                    .borrow_mut()
                                    .on_mute_change(listener_id, *enabled);
                            }
                        } else {
                            error!("Parameter Parse Error, Message was not of expected type");
                            debug!("Object Id: {}", object.id);
                            for property in object.properties {
                                debug!("Key: {}, Value: {:?}", property.key, property.value);
                            }
                        }
                    } else {
                        error!("Unexpected Value Type");
                    }
                }
            })
            .register();
        proxy.subscribe_params(&[ParamType::Props]);

        let node = ManagedNode {
            pw_id: None,
            object_serial: None,
            id: properties.node_id,
            props: node_properties.clone(),
            proxy,
            _listener: listener,
            _proxy_listener: proxy_listener,

            port_map: Default::default(),
            ports_ready: false,

            node_state: NodeStoreState::Creating,
            ready_sender: Some(properties.ready_sender),
        };

        self.managed_node_add(node);

        Ok(())
    }

    pub fn managed_node_add(&mut self, node: ManagedNode) {
        debug!("[{}] Device Added to Store, waiting for data", node.id);
        self.managed_nodes.insert(node.id, node);
    }

    pub fn managed_node_get(&self, id: Ulid) -> Option<&ManagedNode> {
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

    pub fn managed_node_state_changed(&mut self, id: Ulid, state: NodeStoreState) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");
        debug!("Node State Changed to: {:?}", state);

        if let NodeStoreState::Error(error) = &state {
            error!("Node {} entered error state: {}", id, error);
        }

        node.node_state = state;
        self.managed_node_check_ready(id);
    }

    pub fn managed_node_request_ports(&self, id: Ulid) {
        let node = self.managed_nodes.get(&id).expect("Broke");
        node.proxy
            .enum_params(0, Some(ParamType::PortConfig), 0, u32::MAX);
    }

    pub fn managed_node_add_port(&mut self, id: Ulid, location: PortLocation, port_id: u32) {
        let node = self.managed_nodes.get_mut(&id).expect("Broke");

        // If we get here, all our ports have been set, trigger the ready event
        if node.add_port(location, port_id) {
            self.managed_node_ports_ready(id);
        }
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

        if node.is_ready()
            && let Some(sender) = node.ready_sender.take()
        {
            debug!("[{}] Device Ready, sending callback", id);
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

        super::utils::send_volume(&node.proxy, volume);
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

        super::utils::send_mute(&node.proxy, muted);
        Ok(())
    }

    pub fn on_mute_change(&mut self, id: Ulid, muted: bool) {
        let _ = self
            .callback_tx
            .send(PipewireReceiver::NodeMuteChanged(id, muted));
    }
}
