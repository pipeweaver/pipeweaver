use crate::registry::device_node::RegistryDeviceNode;
use crate::registry::port::RegistryPort;
use crate::store::Store;
use crate::{Direction, MediaClass, PipewireReceiver};
use anyhow::bail;
use log::{debug, warn};

impl Store {
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
        let Some(node) = self.unmanaged_device_nodes.get(&id) else {
            return;
        };

        if node.ports_desynced() {
            if node.sent_upstream {
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

        if node.ports_desynced() {
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
            let new_usability = node.usable_media_class().is_some();
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

        let is_usable = node.usable_media_class().is_some();
        let device_node = node.to_device_node(id, media_class, is_usable);

        // Mark as sent BEFORE sending to prevent race conditions
        if let Some(node) = self.unmanaged_device_nodes.get_mut(&id) {
            node.sent_upstream = true;
        }

        let _ = self
            .callback_tx
            .send(PipewireReceiver::DeviceAdded(device_node));
    }

    pub fn unmanaged_node_set_volume(&mut self, id: u32, volume: u8) -> anyhow::Result<()> {
        let Some(node) = self.unmanaged_device_nodes.get(&id) else {
            bail!("Node not found")
        };

        let Some(parent) = node.parent_id.and_then(|pid| self.unmanaged_devices.get(&pid)) else {
            // No parent, set directly on the node
            node.set_volume(volume);
            return Ok(());
        };

        let Some(node_profile_port) = node.profile_port() else {
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

    pub fn unmanaged_node_set_mute(&mut self, id: u32, muted: bool) -> anyhow::Result<()> {
        let Some(node) = self.unmanaged_device_nodes.get(&id) else {
            bail!("Node not found")
        };

        let Some(parent) = node.parent_id.and_then(|pid| self.unmanaged_devices.get(&pid)) else {
            // No parent, set directly on the node
            node.set_mute(muted);
            return Ok(());
        };

        let Some(node_profile_port) = node.profile_port() else {
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
