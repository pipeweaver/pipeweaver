//! Lifecycle of the nodes *we* create (as opposed to `unmanaged_devices`,
//! which tracks nodes pipewire owns): registration, waiting for pipewire to
//! assign an id and ports, state tracking, and volume/mute control for both
//! our own nodes and unmanaged client (application) nodes.

use super::{NodeStore, NodeStoreState, PortLocation, Store};
use crate::PipewireReceiver;
use anyhow::{anyhow, Result};
use log::{debug, error};
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{object, Pod, Property, Value, ValueArray};
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};
use pipewire::spa::utils;
use std::io::Cursor;
use strum::IntoEnumIterator;
use ulid::Ulid;

impl Store {
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
            && !matches!(
                node.node_state,
                NodeStoreState::Creating | NodeStoreState::Error(_)
            )
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
}
