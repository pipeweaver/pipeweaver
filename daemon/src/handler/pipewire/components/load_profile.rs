use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use shared::NodeType;

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
}

impl LoadProfileLocal for PipewireManager {
    async fn profile_create_nodes(&mut self) -> Result<()> {
        // Ok, iterate through the profile device node types, and make them
        for device in self.profile.devices.sources.physical_devices.clone() {
            self.node_create(NodeType::PhysicalSource, &device.description).await?
        }

        for device in self.profile.devices.sources.virtual_devices.clone() {
            self.node_create(NodeType::VirtualSource, &device.description).await?
        }

        for device in self.profile.devices.targets.physical_devices.clone() {
            self.node_create(NodeType::PhysicalTarget, &device.description).await?
        }

        for device in self.profile.devices.targets.virtual_devices.clone() {
            self.node_create(NodeType::VirtualTarget, &device.description).await?
        }

        Ok(())
    }

    async fn profile_load_volumes(&mut self) -> Result<()> {
        self.volumes_load().await
    }

    async fn profile_apply_routing(&mut self) -> Result<()> {
        self.routing_load().await
    }
}