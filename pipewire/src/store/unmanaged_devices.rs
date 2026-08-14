//! Devices and device-nodes that pipewire told us about but that we didn't
//! create ourselves (physical/virtual hardware endpoints, as opposed to
//! clients/applications - see `unmanaged_clients`). The bulk of this module
//! is the "is this node fully synced and usable yet" state machine in
//! [`Store::unmanaged_node_port_check`] and its supporting functions.

use super::Store;
use crate::registry::device::{ActiveRoute, RegistryDevice};
use crate::registry::device_node::RegistryDeviceNode;
use crate::registry::port::RegistryPort;
use crate::{DeviceNode, Direction, MediaClass, NodePort, PipewireReceiver};
use anyhow::{anyhow, bail, Result};
use enum_map::EnumMap;
use log::{debug, warn};
use strum::IntoEnumIterator;

impl Store {
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

    pub fn unmanaged_device_set_active_route(
        &mut self,
        device_id: u32,
        route_device: u32,
        index: u32,
        n_channels: u32,
    ) {
        if let Some(device) = self.unmanaged_devices.get_mut(&device_id) {
            device
                .active_routes
                .insert(route_device, ActiveRoute { index, n_channels });
        }
    }

    pub fn unmanaged_device_node_volume_changed(
        &mut self,
        device_id: u32,
        route_device: u32,
        volume: u8,
    ) {
        let device_nodes = self
            .unmanaged_devices
            .get(&device_id)
            .map(|d| d.nodes.clone());

        let Some(device_nodes) = device_nodes else {
            return;
        };

        // First, try to match by profile_port
        for &node_id in &device_nodes {
            if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id)
                && node.profile_port() == Some(route_device)
            {
                node.volume = volume;

                if node.sent_upstream {
                    let message = PipewireReceiver::DeviceVolumeChanged(node_id, volume);
                    let _ = self.callback_tx.send(message);
                }
                return;
            }
        }

        // Fallback: if the device only has one node, use it directly
        if device_nodes.len() == 1 {
            let node_id = device_nodes[0];
            if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id) {
                node.volume = volume;

                if node.sent_upstream {
                    let _ = self
                        .callback_tx
                        .send(PipewireReceiver::DeviceVolumeChanged(node_id, volume));
                }
            }
        }
    }

    pub fn unmanaged_device_node_mute_changed(
        &mut self,
        device_id: u32,
        route_device: u32,
        muted: bool,
    ) {
        let device_nodes = self
            .unmanaged_devices
            .get(&device_id)
            .map(|d| d.nodes.clone());

        let Some(device_nodes) = device_nodes else {
            return;
        };

        // First, try to match by profile_port
        for &node_id in &device_nodes {
            if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id)
                && node.muted != muted
                && node.profile_port() == Some(route_device)
            {
                node.muted = muted;

                if node.sent_upstream {
                    let message = PipewireReceiver::DeviceMuteChanged(node_id, muted);
                    let _ = self.callback_tx.send(message);
                }
                return;
            }
        }

        // Fallback: if the device only has one node, use it directly
        if device_nodes.len() == 1 {
            let node_id = device_nodes[0];
            if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id)
                && node.muted != muted
            {
                node.muted = muted;

                if node.sent_upstream {
                    let _ = self
                        .callback_tx
                        .send(PipewireReceiver::DeviceMuteChanged(node_id, muted));
                }
            }
        }
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

    pub fn add_pending_device_sync(&mut self, seq: i32, id: u32) {
        self.pending_device_syncs.insert(seq, id);
    }

    // pub fn resolve_pending_device_sync(&mut self, seq: i32) {
    //     if let Some(id) = self.pending_device_syncs.remove(&seq)
    //         && let Some(node) = self.unmanaged_device_nodes.get_mut(&id)
    //     {
    //         debug!("Device Synced, Checking.. {}", id);
    //         node.is_synced = true;
    //         self.unmanaged_node_port_check(id);
    //     }
    // }

    pub fn unmanaged_node_port_count_update(&mut self, id: u32, in_count: u32, out_count: u32) {
        let node = match self.unmanaged_device_nodes.get_mut(&id) {
            Some(node) => node,
            None => return,
        };

        let current_in = node.port_count[Direction::In];
        let current_out = node.port_count[Direction::Out];
        if current_in == Some(in_count) && current_out == Some(out_count) {
            // Nothing has changed, nothing to do.
            return;
        }
        debug!(
            "Node {} port count updated (In: {:?} -> {}, Out: {:?} -> {})",
            id, current_in, in_count, current_out, out_count
        );

        node.port_count[Direction::In] = Some(in_count);
        node.port_count[Direction::Out] = Some(out_count);

        self.unmanaged_node_reconcile(id);
    }

    pub fn unmanaged_node_port_removed(&mut self, node_id: u32, dir: Direction, port: u32) {
        if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id) {
            node.ports[dir].remove(&port);
        }

        self.unmanaged_node_reconcile(node_id);
    }

    pub fn unmanaged_node_port_add(&mut self, node_id: u32, dir: Direction, port: RegistryPort) {
        if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id) {
            node.add_port(dir, port);
        }

        self.unmanaged_node_reconcile(node_id);
    }

    fn unmanaged_node_reconcile(&mut self, id: u32) {
        let (is_desynced, was_sent_upstream) = match self.unmanaged_device_nodes.get(&id) {
            Some(node) => (self.unmanaged_node_is_desynced(id), node.sent_upstream),
            None => return,
        };

        if is_desynced {
            if was_sent_upstream {
                let _ = self.callback_tx.send(PipewireReceiver::DeviceRemoved(id));

                if let Some(node) = self.unmanaged_device_nodes.get_mut(&id) {
                    node.sent_upstream = false;
                }
            }

            return;
        }

        // If we're synced, try progressing state
        self.unmanaged_node_port_check(id);
    }

    pub fn unmanaged_node_is_desynced(&self, node_id: u32) -> bool {
        if let Some(node) = self.unmanaged_device_nodes.get(&node_id) {
            for direction in Direction::iter() {
                if node.port_count[direction].is_none() {
                    return true;
                }

                if Some(node.ports[direction].len() as u32) != node.port_count[direction] {
                    return true;
                }
            }
        } else {
            return true;
        }

        false
    }

    pub fn unmanaged_node_set_clock_ready(&mut self, id: u32) -> bool {
        if let Some(node) = self.unmanaged_device_nodes.get_mut(&id)
            && !node.clock_ready
        {
            node.clock_ready = true;
            debug!("Node {} clock is now ready", id);
            self.unmanaged_node_port_check(id);
            return true;
        }
        false
    }

    pub fn unmanaged_node_port_check(&mut self, id: u32) {
        // Check if a node is ready to be sent upstream or needs an update.
        // Called when:
        // - A port is added
        // - A port is removed
        // - Port count info is updated (via unmanaged_node_port_count_update)

        let node = if let Some(node) = self.unmanaged_device_nodes.get(&id) {
            node
        } else {
            return;
        };

        if !node.clock_ready {
            return;
        }

        if !node.is_synced {
            return;
        }

        // Check if we have port count expectations for both directions
        let has_port_count_info =
            node.port_count[Direction::In].is_some() && node.port_count[Direction::Out].is_some();

        if !has_port_count_info {
            debug!("Node {} missing port count info, waiting...", id);
            return;
        }

        // Check if received port count matches expected count
        let mut is_complete = true;
        for direction in Direction::iter() {
            let count = node.ports[direction].len();
            if node.port_count[direction] != Some(count as u32) {
                is_complete = false;
                break;
            }
        }

        if !is_complete {
            debug!(
                "Node {} ports incomplete (In: {} of {:?}, Out: {} of {:?}), waiting...",
                id,
                node.ports[Direction::In].len(),
                node.port_count[Direction::In],
                node.ports[Direction::Out].len(),
                node.port_count[Direction::Out]
            );
            return;
        }

        // Ports are complete - either send initial or update
        if node.sent_upstream {
            // Already sent, check if usability changed
            let new_usability = self.is_usable_unmanaged_device_node(id).is_some();
            debug!(
                "Node {} port configuration complete, updating usability: {}",
                id, new_usability
            );
            let _ = self
                .callback_tx
                .send(PipewireReceiver::DeviceUsable(id, new_usability));
        } else {
            // Not sent yet, send it now
            debug!("Port Count Matches for Node: {}, Sending Device..", id);
            self.unmanaged_node_send(id);
        }
    }

    pub fn unmanaged_node_send(&mut self, id: u32) {
        // Check if the node exists and hasn't been sent yet
        let node = if let Some(node) = self.unmanaged_device_nodes.get(&id) {
            if node.sent_upstream {
                return;
            }
            node
        } else {
            return;
        };

        // We need a media class, otherwise we can't use this node
        let Some(media_class_str) = &node.media_class else {
            return;
        };

        // Map the media class to our internal enum
        let media_class = match media_class_str.as_str() {
            s if s.starts_with("Audio/Sink") => Some(MediaClass::Sink),
            s if s.starts_with("Audio/Source") => Some(MediaClass::Source),
            s if s.starts_with("Audio/Duplex") => Some(MediaClass::Duplex),
            _ => {
                warn!("Unrecognized Media Class: {}", media_class_str);
                None
            }
        };

        let Some(media_class) = media_class else {
            return;
        };

        let is_usable = self.is_usable_unmanaged_device_node(id).is_some();

        let mut ports: EnumMap<Direction, Vec<NodePort>> = Default::default();
        for direction in Direction::iter() {
            for (_, port) in node.ports[direction].iter() {
                // Don't send Monitor ports
                if !port.is_monitor {
                    ports[direction].push(NodePort {
                        name: port.name.clone(),
                        channel: port.channel.clone(),
                    });
                }
            }
        }

        // Create the virtual node and send it upstream
        let device_node = DeviceNode {
            node_id: id,
            node_class: media_class,
            is_usable,
            name: node.name.clone(),
            nickname: node.nickname.clone(),
            description: node.description.clone(),

            volume: node.volume,
            muted: node.muted,

            ports,
        };

        // Mark as sent BEFORE sending to prevent race conditions
        if let Some(node) = self.unmanaged_device_nodes.get_mut(&id) {
            node.sent_upstream = true;
        }

        let _ = self
            .callback_tx
            .send(PipewireReceiver::DeviceAdded(device_node));
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
                let non_monitor: Vec<_> = ports.values().filter(|p| !p.is_monitor).collect();
                let count = if non_monitor.len() > 2 {
                    // We should consider things like 5.1 devices valid, so long as there's a FL / FR
                    let has_left = non_monitor
                        .iter()
                        .any(|p| p.channel == "FL" || p.channel == "AUX0");
                    let has_right = non_monitor
                        .iter()
                        .any(|p| p.channel == "FR" || p.channel == "AUX1");

                    // If we have them, force this count to 2, which will pass get_media_class
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

            return self.get_media_class(in_count, out_count);
        }
        None
    }

    pub fn unmanaged_node_set_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        let Some(node) = self.unmanaged_device_nodes.get(&id) else {
            bail!("Node not found")
        };

        let Some(parent) = self
            .unmanaged_devices
            .values()
            .find(|d| d.nodes.contains(&id))
        else {
            // No parent, set directly on the node
            node.set_volume(volume);
            return Ok(());
        };

        let node_port = self
            .unmanaged_device_nodes
            .get(&id)
            .ok_or_else(|| anyhow!("Node not found"))?
            .profile_port();

        let Some(node_profile_port) = node_port else {
            return Ok(());
        };

        let linear_vol = (volume as f32 / 100.0).powi(3);
        for (route_dev, route) in &parent.active_routes {
            if route_dev == &node_profile_port {
                parent.set_volume(*route_dev, route.index, route.n_channels, linear_vol)?;
            }
        }

        Ok(())
    }

    pub fn unmanaged_node_set_mute(&mut self, id: u32, muted: bool) -> Result<()> {
        let Some(node) = self.unmanaged_device_nodes.get(&id) else {
            bail!("Node not found")
        };

        let Some(parent) = self
            .unmanaged_devices
            .values()
            .find(|d| d.nodes.contains(&id))
        else {
            // No parent, set directly on the node
            node.set_mute(muted);
            return Ok(());
        };

        let node_port = self
            .unmanaged_device_nodes
            .get(&id)
            .ok_or_else(|| anyhow!("Node not found"))?
            .profile_port();

        let Some(node_profile_port) = node_port else {
            return Ok(());
        };

        for (route_dev, route) in &parent.active_routes {
            if route_dev == &node_profile_port {
                parent.set_mute(*route_dev, route.index, muted)?;
            }
        }
        Ok(())
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
}
