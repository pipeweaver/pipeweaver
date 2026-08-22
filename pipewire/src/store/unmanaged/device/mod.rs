use crate::PipewireReceiver;
use crate::registry::device::{ActiveRoute, RegistryDevice};
use crate::store::Store;

pub(super) mod node;

impl Store {
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
}
