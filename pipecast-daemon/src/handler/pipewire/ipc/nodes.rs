use crate::handler::pipewire::manager::PipewireManager;
use pipecast_profile::{PhysicalSourceDevice, VirtualSourceDevice};

pub(crate) trait NodeManager {
    async fn create_virtual_source(&mut self, device: &VirtualSourceDevice);
    async fn create_physical_source(&mut self, device: &PhysicalSourceDevice);
}

impl NodeManager for PipewireManager {
    async fn create_virtual_source(&mut self, device: &VirtualSourceDevice) {
        todo!()
    }

    async fn create_physical_source(&mut self, device: &PhysicalSourceDevice) {
        todo!()
    }
}


/// This is a Private implementation of methods which may be needed to be related to Nodes
trait NodeManagerLocal {}

impl NodeManagerLocal for PipewireManager {}