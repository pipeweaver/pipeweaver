use crate::registry::client_node::RegistryClientNode;
use crate::registry::port::RegistryPort;
use crate::store::{Store, TargetType};
use crate::{Direction, NodeTarget, PipewireReceiver};
use anyhow::{Result, bail};
use log::{debug, error, warn};
use pipewire::node::NodeState;
use std::mem::discriminant;

impl Store {
    pub fn unmanaged_client_node_add(&mut self, id: u32, node: RegistryClientNode) {
        self.unmanaged_client_nodes.insert(id, node);
    }

    pub fn unmanaged_client_node_get(&mut self, id: u32) -> Option<&mut RegistryClientNode> {
        self.unmanaged_client_nodes.get_mut(&id)
    }

    pub fn unmanaged_client_node_port_add(&mut self, id: u32, dir: Direction, port: RegistryPort) {
        if let Some(node) = self.unmanaged_client_node_get(id) {
            node.add_port(dir, port);
            self.unmanaged_client_node_check(id);
        }
    }

    // These are incoming from the User
    pub fn set_application_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        let Some(node) = self.unmanaged_client_node_get(id) else {
            bail!("Failed to find node");
        };

        node.set_volume(volume);
        Ok(())
    }
    pub fn set_application_muted(&mut self, id: u32, muted: bool) -> Result<()> {
        let Some(node) = self.unmanaged_client_node_get(id) else {
            bail!("Failed to find node");
        };

        node.set_mute(muted);
        Ok(())
    }

    // Received a Volume change from Pipewire
    pub fn unmanaged_client_node_recv_volume(&mut self, id: u32, volume: u8) {
        if let Some(node) = self.unmanaged_client_node_get(id)
            && node.volume != volume
        {
            node.volume = volume;

            let msg = PipewireReceiver::ApplicationVolumeChanged(id, volume);
            let _ = self.callback_tx.send(msg);
        }
    }

    // Received a Mute state from Pipewire
    pub fn unmanaged_client_node_recv_mute(&mut self, id: u32, muted: bool) {
        if let Some(node) = self.unmanaged_client_node_get(id) {
            if node.is_muted != muted {
                node.is_muted = muted;

                let msg = PipewireReceiver::ApplicationMuteChanged(id, muted);
                let _ = self.callback_tx.send(msg);
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
            let msg = PipewireReceiver::ApplicationRemoved(id);
            let _ = self.callback_tx.send(msg);
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
            && let Some(parent) = self.unmanaged_clients.get(&node.parent_id)
            && let Some(media_type) = node.is_usable(parent)
        {
            self.usable_client_nodes.push(id);
            let node = node.to_node(id, media_type, parent);

            let message = PipewireReceiver::ApplicationAdded(node);
            let _ = self.callback_tx.send(message);
        }
    }
}
