use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use log::debug;
use pipeweaver_profile::DeviceDescription;
use pipeweaver_shared::{NodeType, OrderGroup};

pub(crate) trait LoadProfile {
    async fn load_profile(&mut self) -> Result<()>;
}

impl LoadProfile for PipewireManager {
    async fn load_profile(&mut self) -> Result<()> {
        self.profile_create_nodes().await?;
        self.profile_load_volumes().await?;
        self.profile_apply_routing().await?;

        Ok(())
    }
}

trait LoadProfileLocal {
    async fn profile_create_nodes(&mut self) -> Result<()>;
    async fn profile_load_volumes(&mut self) -> Result<()>;
    async fn profile_apply_routing(&mut self) -> Result<()>;
    fn check_device_order_present(&mut self, dev: &DeviceDescription, source: bool) -> Result<()>;
    fn validate_device_order(&mut self, source: bool) -> Result<()>;
}

impl LoadProfileLocal for PipewireManager {
    async fn profile_create_nodes(&mut self) -> Result<()> {
        // Ok, iterate through the profile device node types, and make them
        for device in self.profile.devices.sources.physical_devices.clone() {
            self.node_create(NodeType::PhysicalSource, &device.description)
                .await?;
            self.check_device_order_present(&device.description, true)?;
        }

        for device in self.profile.devices.sources.virtual_devices.clone() {
            self.node_create(NodeType::VirtualSource, &device.description)
                .await?;
            self.check_device_order_present(&device.description, true)?;
        }
        self.validate_device_order(true)?;


        for device in self.profile.devices.targets.physical_devices.clone() {
            self.node_create(NodeType::PhysicalTarget, &device.description)
                .await?;
            self.check_device_order_present(&device.description, false)?;
        }

        for device in self.profile.devices.targets.virtual_devices.clone() {
            self.node_create(NodeType::VirtualTarget, &device.description)
                .await?;
            self.check_device_order_present(&device.description, false)?;
        }
        self.validate_device_order(false)?;

        Ok(())
    }

    async fn profile_load_volumes(&mut self) -> Result<()> {
        self.volumes_load().await
    }

    async fn profile_apply_routing(&mut self) -> Result<()> {
        self.routing_load().await
    }

    fn check_device_order_present(&mut self, dev: &DeviceDescription, source: bool) -> Result<()> {
        let order_list = if source {
            &mut self.profile.devices.sources.device_order
        } else {
            &mut self.profile.devices.targets.device_order
        };

        if !order_list.iter().any(|(_, v)| v.contains(&dev.id)) {
            debug!("Device Not Found in Order List, adding to Default List");
            order_list[OrderGroup::default()].push(dev.id);
        }

        Ok(())
    }

    fn validate_device_order(&mut self, source: bool) -> Result<()> {
        // We're looking for devices which may be present in the order, but don't exist
        let mut known_ids = vec![];
        let device_order = if source {
            for device in &self.profile.devices.sources.physical_devices {
                known_ids.push(device.description.id);
            }
            for device in &self.profile.devices.sources.virtual_devices {
                known_ids.push(device.description.id);
            }
            &mut self.profile.devices.sources.device_order
        } else {
            for device in &self.profile.devices.targets.physical_devices {
                known_ids.push(device.description.id);
            }
            for device in &self.profile.devices.targets.virtual_devices {
                known_ids.push(device.description.id);
            }
            &mut self.profile.devices.targets.device_order
        };

        // We'll use a .retain on each element of the order to clean up
        for vec in device_order.values_mut() {
            vec.retain(|id| known_ids.contains(id));
        }
        Ok(())
    }
}
