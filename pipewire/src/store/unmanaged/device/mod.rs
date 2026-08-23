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
        let Some(device) = self.unmanaged_devices.get(&device_id) else {
            return;
        };

        let Some(node_id) = device.resolve_route(route_device, &self.unmanaged_device_nodes) else {
            return;
        };

        if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id) {
            node.volume = volume;

            if node.sent_upstream {
                let message = PipewireReceiver::DeviceVolumeChanged(node_id, volume);
                let _ = self.callback_tx.send(message);
            }
        }
    }

    pub fn unmanaged_device_node_mute_changed(&mut self, dev_id: u32, route_dev: u32, muted: bool) {
        let Some(device) = self.unmanaged_devices.get(&dev_id) else {
            return;
        };

        let Some(node_id) = device.resolve_route(route_dev, &self.unmanaged_device_nodes) else {
            return;
        };

        if let Some(node) = self.unmanaged_device_nodes.get_mut(&node_id)
            && node.muted != muted
        {
            node.muted = muted;

            if node.sent_upstream {
                let message = PipewireReceiver::DeviceMuteChanged(node_id, muted);
                let _ = self.callback_tx.send(message);
            }
        }
    }

    pub fn unmanaged_device_remove(&mut self, id: u32) {
        self.unmanaged_devices.remove(&id);
    }
}
