//! Pipewire only tells us an id has gone away, never what *kind* of object
//! it was - so `remove_by_id` has to check every collection we hold in turn
//! and route to the right removal handler.

use super::Store;
use crate::{Direction, PipewireReceiver};
use log::{debug, trace};
use strum::IntoEnumIterator;

impl Store {
    pub fn remove_by_id(&mut self, id: u32) {
        if self.unmanaged_devices.contains_key(&id) {
            trace!("Removing Unmanaged Device: {}", id);
            return self.unmanaged_device_remove(id);
        }

        if self.unmanaged_device_nodes.contains_key(&id) {
            trace!("Removing Unmanaged Nodes: {}", id);
            return self.unmanaged_device_node_remove(id);
        }

        if self.unmanaged_clients.contains_key(&id) {
            trace!("Removing Unmanaged Client: {}", id);
            return self.unmanaged_client_remove(id);
        }

        if self.unmanaged_client_nodes.contains_key(&id) {
            trace!("Removing Unmanaged Client Node: {}", id);
            return self.unmanaged_client_node_remove(id);
        }

        if self.unmanaged_links.contains_key(&id) {
            trace!("Removing Unmanaged Links: {}", id);
            return self.unmanaged_link_remove(id);
        }

        // Something may be trying to mess with a managed link, if so, completely drop our links
        // and report back to whatever is calling us that it's happened, so they can action it.
        if let Some(id) = self.is_managed_link(id) {
            debug!("Removing Managed Link: {}", id);
            if let Some(link) = self.managed_links.remove(&id) {
                debug!("Removed Links: {:?} -> {:?}", link.source, link.destination);
                let _ = self.callback_tx.send(PipewireReceiver::ManagedLinkDropped(
                    link.source,
                    link.destination,
                ));
            }
        }

        if let Some(id) = self.is_managed_link(id)
            && let Some(link) = self.managed_links.remove(&id)
        {
            let _ = self.callback_tx.send(PipewireReceiver::ManagedLinkDropped(
                link.source,
                link.destination,
            ));
        }

        // This might be a port removal from an unmanaged node
        struct NodePort {
            node_id: u32,
            direction: Direction,
            port_id: u32,
        }
        let mut nodes_to_check = Vec::new();
        for (node_id, node) in self.unmanaged_device_nodes.iter_mut() {
            for direction in Direction::iter() {
                if node.ports[direction].contains_key(&id) {
                    nodes_to_check.push(NodePort {
                        node_id: *node_id,
                        direction,
                        port_id: id,
                    });
                }
            }
        }
        for node in nodes_to_check {
            self.unmanaged_node_port_removed(node.node_id, node.direction, node.port_id);
        }
    }
}
