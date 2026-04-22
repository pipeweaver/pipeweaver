use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{Result, bail};
use pipeweaver_pipewire::PipewireMessage::SetDefaultDevice;
use pipeweaver_pipewire::{MediaClass, NodeTarget};
use pipeweaver_shared::{DeviceType, NodeType};
use ulid::Ulid;

pub(crate) trait DefaultHandlers {
    async fn set_default_input(&self, id: Ulid) -> Result<()>;
    async fn set_default_output(&self, id: Ulid) -> Result<()>;
}

impl DefaultHandlers for PipewireManager {
    async fn set_default_input(&self, id: Ulid) -> Result<()> {
        self.set_default_device(id, MediaClass::Source).await
    }

    async fn set_default_output(&self, id: Ulid) -> Result<()> {
        self.set_default_device(id, MediaClass::Sink).await
    }
}

trait DefaultHandlersInternal {
    fn find_physical_device(&self, id: Ulid, device_type: DeviceType) -> Option<u32>;
    async fn set_default_device(&self, id: Ulid, class: MediaClass) -> Result<()>;
}

impl DefaultHandlersInternal for PipewireManager {
    fn find_physical_device(&self, id: Ulid, device_type: DeviceType) -> Option<u32> {
        let mut result = None;

        // We need to check physical nodes
        for node in &self.node_list[device_type] {
            if node.id == id {
                result = Some(node.node_id);
                break;
            }
        }

        if let Some(node_id) = result {
            return Some(node_id);
        }

        None
    }

    async fn set_default_device(&self, id: Ulid, class: MediaClass) -> Result<()> {
        let (valid_types, device_type) = match class {
            MediaClass::Source => (
                &[NodeType::PhysicalSource, NodeType::VirtualSource] as &[_],
                DeviceType::Source,
            ),
            MediaClass::Sink => (
                &[NodeType::PhysicalTarget, NodeType::VirtualTarget] as &[_],
                DeviceType::Target,
            ),
            MediaClass::Duplex => bail!("Duplex is not a valid default device class"),
        };

        let target = if let Some(node) = self.get_node_type(id) {
            if valid_types.contains(&node) {
                NodeTarget::Node(id)
            } else {
                bail!("Invalid Node Type");
            }
        } else if let Some(dev) = self.find_physical_device(id, device_type) {
            NodeTarget::UnmanagedNode(dev)
        } else {
            bail!("No node or device found with the given ID");
        };

        let message = SetDefaultDevice(class, target);
        self.pipewire().send_message(message)?;
        Ok(())
    }
}
