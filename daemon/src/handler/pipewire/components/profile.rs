use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use anyhow::anyhow;
use pipeweaver_profile::{
    DeviceDescription, PhysicalSourceDevice, PhysicalTargetDevice, VirtualSourceDevice,
    VirtualTargetDevice,
};
use pipeweaver_shared::NodeType;
use ulid::Ulid;

pub(crate) trait ProfileManagement {
    fn get_physical_source(&self, id: Ulid) -> Option<&PhysicalSourceDevice>;
    fn get_physical_source_mut(&mut self, id: Ulid) -> Option<&mut PhysicalSourceDevice>;

    fn get_virtual_source(&self, id: Ulid) -> Option<&VirtualSourceDevice>;
    fn get_virtual_source_mut(&mut self, id: Ulid) -> Option<&mut VirtualSourceDevice>;

    fn get_physical_target(&self, id: Ulid) -> Option<&PhysicalTargetDevice>;
    fn get_physical_target_mut(&mut self, id: Ulid) -> Option<&mut PhysicalTargetDevice>;

    fn get_virtual_target(&self, id: Ulid) -> Option<&VirtualTargetDevice>;
    fn get_virtual_target_mut(&mut self, id: Ulid) -> Option<&mut VirtualTargetDevice>;

    fn get_device_description(&mut self, id: Ulid) -> Result<&mut DeviceDescription>;
}

impl ProfileManagement for PipewireManager {
    fn get_physical_source(&self, id: Ulid) -> Option<&PhysicalSourceDevice> {
        self.profile
            .devices
            .sources
            .physical_devices
            .iter()
            .find(|d| d.description.id == id)
    }

    fn get_physical_source_mut(&mut self, id: Ulid) -> Option<&mut PhysicalSourceDevice> {
        self.profile
            .devices
            .sources
            .physical_devices
            .iter_mut()
            .find(|d| d.description.id == id)
    }

    fn get_virtual_source(&self, id: Ulid) -> Option<&VirtualSourceDevice> {
        self.profile
            .devices
            .sources
            .virtual_devices
            .iter()
            .find(|d| d.description.id == id)
    }

    fn get_virtual_source_mut(&mut self, id: Ulid) -> Option<&mut VirtualSourceDevice> {
        self.profile
            .devices
            .sources
            .virtual_devices
            .iter_mut()
            .find(|d| d.description.id == id)
    }

    fn get_physical_target(&self, id: Ulid) -> Option<&PhysicalTargetDevice> {
        self.profile
            .devices
            .targets
            .physical_devices
            .iter()
            .find(|d| d.description.id == id)
    }

    fn get_physical_target_mut(&mut self, id: Ulid) -> Option<&mut PhysicalTargetDevice> {
        self.profile
            .devices
            .targets
            .physical_devices
            .iter_mut()
            .find(|d| d.description.id == id)
    }

    fn get_virtual_target(&self, id: Ulid) -> Option<&VirtualTargetDevice> {
        self.profile
            .devices
            .targets
            .virtual_devices
            .iter()
            .find(|d| d.description.id == id)
    }

    fn get_virtual_target_mut(&mut self, id: Ulid) -> Option<&mut VirtualTargetDevice> {
        self.profile
            .devices
            .targets
            .virtual_devices
            .iter_mut()
            .find(|d| d.description.id == id)
    }

    fn get_device_description(&mut self, id: Ulid) -> Result<&mut DeviceDescription> {
        let err = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;

        let err = anyhow!("Failed to Get Node Type");
        match node_type {
            NodeType::PhysicalSource => {
                Ok(&mut self.get_physical_source_mut(id).ok_or(err)?.description)
            }
            NodeType::PhysicalTarget => {
                Ok(&mut self.get_physical_target_mut(id).ok_or(err)?.description)
            }
            NodeType::VirtualSource => {
                Ok(&mut self.get_virtual_source_mut(id).ok_or(err)?.description)
            }
            NodeType::VirtualTarget => {
                Ok(&mut self.get_virtual_target_mut(id).ok_or(err)?.description)
            }
        }
    }
}
