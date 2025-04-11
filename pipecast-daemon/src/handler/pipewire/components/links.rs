use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use pipecast_pipewire::{LinkType, PipewireMessage};
use tokio::sync::oneshot;
use ulid::Ulid;

/// So this trait is INCREDIBLY verbose, I could simply just use LinkType and have a single function
/// but from a readability perspective having incoming calls define exactly what they want to do
/// and managing that accordingly ensures clean and defined behaviour.
pub(crate) trait LinkManagement {
    async fn link_create_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()>;

    async fn link_create_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;

    async fn link_create_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;

    async fn link_create_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()>;


    async fn link_remove_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()>;

    async fn link_remove_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;

    async fn link_remove_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;

    async fn link_remove_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()>;
}

impl LinkManagement for PipewireManager {
    async fn link_create_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()> {
        self.create_link(source, target).await
    }
    async fn link_create_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Node(source), LinkType::Node(target)).await
    }
    async fn link_create_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Node(source), LinkType::Filter(target)).await
    }
    async fn link_create_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.create_link(LinkType::Node(source), LinkType::UnmanagedNode(target)).await
    }

    async fn link_create_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Filter(source), LinkType::Node(target)).await
    }
    async fn link_create_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Filter(source), LinkType::Filter(target)).await
    }
    async fn link_create_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.create_link(LinkType::Filter(source), LinkType::UnmanagedNode(target)).await
    }

    async fn link_create_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()> {
        self.create_link(LinkType::UnmanagedNode(source), LinkType::Node(target)).await
    }
    async fn link_create_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()> {
        self.create_link(LinkType::UnmanagedNode(source), LinkType::Filter(target)).await
    }
    async fn link_create_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()> {
        self.create_link(LinkType::UnmanagedNode(source), LinkType::UnmanagedNode(target)).await
    }


    async fn link_remove_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()> {
        self.remove_link(source, target).await
    }
    async fn link_remove_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Node(source), LinkType::Node(target)).await
    }
    async fn link_remove_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Node(source), LinkType::Filter(target)).await
    }
    async fn link_remove_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.remove_link(LinkType::Node(source), LinkType::UnmanagedNode(target)).await
    }

    async fn link_remove_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Filter(source), LinkType::Node(target)).await
    }
    async fn link_remove_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Filter(source), LinkType::Filter(target)).await
    }
    async fn link_remove_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.remove_link(LinkType::Filter(source), LinkType::UnmanagedNode(target)).await
    }

    async fn link_remove_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::UnmanagedNode(source), LinkType::Node(target)).await
    }
    async fn link_remove_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::UnmanagedNode(source), LinkType::Filter(target)).await
    }
    async fn link_remove_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()> {
        self.remove_link(LinkType::UnmanagedNode(source), LinkType::UnmanagedNode(target)).await
    }
}

trait LinkManagementLocal {
    async fn create_link(&self, source: LinkType, target: LinkType) -> Result<()>;
    async fn remove_link(&self, source: LinkType, target: LinkType) -> Result<()>;
}

impl LinkManagementLocal for PipewireManager {
    async fn create_link(&self, source: LinkType, target: LinkType) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let message = PipewireMessage::CreateDeviceLink(source, target, Some(send));
        self.pipewire().send_message(message)?;
        recv.await?;

        Ok(())
    }

    async fn remove_link(&self, source: LinkType, target: LinkType) -> Result<()> {
        let message = PipewireMessage::RemoveDeviceLink(source, target);
        self.pipewire().send_message(message)
    }
}