use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use pipeweaver_pipewire::{LinkPorts, oneshot};
use pipeweaver_pipewire::{LinkType, PipewireMessage};
use ulid::Ulid;

/// So this trait is INCREDIBLY verbose, I could simply just use LinkType and have a single function
/// but from a readability perspective having incoming calls define exactly what they want to do
/// and managing that accordingly ensures clean and defined behaviour
#[allow(unused)]
#[rustfmt::skip]
pub(crate) trait LinkManagement {
    async fn link_create_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()>;

    async fn link_create_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;
    async fn link_create_node_to_unmanaged_ports(&self, source: Ulid, target: u32, ports: LinkPorts) -> Result<()>;

    async fn link_create_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_create_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;
    async fn link_create_filter_to_unmanaged_ports(&self, source: Ulid, target: u32, ports: LinkPorts) -> Result<()>;

    async fn link_create_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_ports_to_node(&self, source: u32, ports: LinkPorts, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_ports_to_filter(&self, source: u32, ports: LinkPorts, target: Ulid) -> Result<()>;
    async fn link_create_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()>;
    async fn link_create_unmanaged_ports_to_unmanaged(&self, source: u32, ports: LinkPorts, target: u32) -> Result<()>;
    async fn link_create_unmanaged_ports_to_unmanaged_ports(&self, source: u32, source_ports: LinkPorts, target: u32, target_ports: LinkPorts) -> Result<()>;

    async fn link_remove_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()>;

    async fn link_remove_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;
    async fn link_remove_node_to_unmanaged_ports(&self, source: Ulid, target: u32, ports: LinkPorts) -> Result<()>;

    async fn link_remove_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()>;
    async fn link_remove_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()>;
    async fn link_remove_filter_to_unmanaged_ports(&self, source: Ulid, target: u32, ports: LinkPorts) -> Result<()>;

    async fn link_remove_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_ports_to_node(&self, source: u32, ports: LinkPorts, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_ports_to_filter(&self, source: u32, ports: LinkPorts, target: Ulid) -> Result<()>;
    async fn link_remove_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()>;
    async fn link_remove_unmanaged_ports_to_unmanaged(&self, source: u32, ports: LinkPorts, target: u32) -> Result<()>;
    async fn link_remove_unmanaged_ports_to_unmanaged_ports(&self, source: u32, source_ports: LinkPorts, target: u32, target_ports: LinkPorts) -> Result<()>;
}

impl LinkManagement for PipewireManager {
    async fn link_create_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()> {
        self.create_link(source, target).await
    }
    async fn link_create_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Node(source), LinkType::Node(target))
            .await
    }
    async fn link_create_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Node(source), LinkType::Filter(target))
            .await
    }
    async fn link_create_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.create_link(
            LinkType::Node(source),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_create_node_to_unmanaged_ports(
        &self,
        source: Ulid,
        target: u32,
        ports: LinkPorts,
    ) -> Result<()> {
        self.create_link(
            LinkType::Node(source),
            LinkType::UnmanagedNode(target, Some(ports)),
        )
        .await
    }

    async fn link_create_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Filter(source), LinkType::Node(target))
            .await
    }
    async fn link_create_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.create_link(LinkType::Filter(source), LinkType::Filter(target))
            .await
    }
    async fn link_create_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.create_link(
            LinkType::Filter(source),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_create_filter_to_unmanaged_ports(
        &self,
        source: Ulid,
        target: u32,
        ports: LinkPorts,
    ) -> Result<()> {
        self.create_link(
            LinkType::Filter(source),
            LinkType::UnmanagedNode(target, Some(ports)),
        )
        .await
    }

    async fn link_create_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::Node(target),
        )
        .await
    }

    async fn link_create_unmanaged_ports_to_node(
        &self,
        source: u32,
        ports: LinkPorts,
        target: Ulid,
    ) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::Node(target),
        )
        .await
    }

    async fn link_create_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::Filter(target),
        )
        .await
    }

    async fn link_create_unmanaged_ports_to_filter(
        &self,
        source: u32,
        ports: LinkPorts,
        target: Ulid,
    ) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::Filter(target),
        )
        .await
    }

    async fn link_create_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_create_unmanaged_ports_to_unmanaged(
        &self,
        source: u32,
        ports: LinkPorts,
        target: u32,
    ) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_create_unmanaged_ports_to_unmanaged_ports(
        &self,
        source: u32,
        source_ports: LinkPorts,
        target: u32,
        target_ports: LinkPorts,
    ) -> Result<()> {
        self.create_link(
            LinkType::UnmanagedNode(source, Some(source_ports)),
            LinkType::UnmanagedNode(target, Some(target_ports)),
        )
        .await
    }

    async fn link_remove_type_to_type(&self, source: LinkType, target: LinkType) -> Result<()> {
        self.remove_link(source, target).await
    }
    async fn link_remove_node_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Node(source), LinkType::Node(target))
            .await
    }
    async fn link_remove_node_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Node(source), LinkType::Filter(target))
            .await
    }
    async fn link_remove_node_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.remove_link(
            LinkType::Node(source),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_remove_node_to_unmanaged_ports(
        &self,
        source: Ulid,
        target: u32,
        ports: LinkPorts,
    ) -> Result<()> {
        self.remove_link(
            LinkType::Node(source),
            LinkType::UnmanagedNode(target, Some(ports)),
        )
        .await
    }

    async fn link_remove_filter_to_node(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Filter(source), LinkType::Node(target))
            .await
    }
    async fn link_remove_filter_to_filter(&self, source: Ulid, target: Ulid) -> Result<()> {
        self.remove_link(LinkType::Filter(source), LinkType::Filter(target))
            .await
    }
    async fn link_remove_filter_to_unmanaged(&self, source: Ulid, target: u32) -> Result<()> {
        self.remove_link(
            LinkType::Filter(source),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_remove_filter_to_unmanaged_ports(
        &self,
        source: Ulid,
        target: u32,
        ports: LinkPorts,
    ) -> Result<()> {
        self.remove_link(
            LinkType::Filter(source),
            LinkType::UnmanagedNode(target, Some(ports)),
        )
        .await
    }

    async fn link_remove_unmanaged_to_node(&self, source: u32, target: Ulid) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::Node(target),
        )
        .await
    }

    async fn link_remove_unmanaged_ports_to_node(
        &self,
        source: u32,
        ports: LinkPorts,
        target: Ulid,
    ) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::Node(target),
        )
        .await
    }

    async fn link_remove_unmanaged_to_filter(&self, source: u32, target: Ulid) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::Filter(target),
        )
        .await
    }

    async fn link_remove_unmanaged_ports_to_filter(
        &self,
        source: u32,
        ports: LinkPorts,
        target: Ulid,
    ) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::Filter(target),
        )
        .await
    }

    async fn link_remove_unmanaged_to_unmanaged(&self, source: u32, target: u32) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, None),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_remove_unmanaged_ports_to_unmanaged(
        &self,
        source: u32,
        ports: LinkPorts,
        target: u32,
    ) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, Some(ports)),
            LinkType::UnmanagedNode(target, None),
        )
        .await
    }

    async fn link_remove_unmanaged_ports_to_unmanaged_ports(
        &self,
        source: u32,
        source_ports: LinkPorts,
        target: u32,
        target_ports: LinkPorts,
    ) -> Result<()> {
        self.remove_link(
            LinkType::UnmanagedNode(source, Some(source_ports)),
            LinkType::UnmanagedNode(target, Some(target_ports)),
        )
        .await
    }
}

trait LinkManagementLocal {
    async fn create_link(&self, source: LinkType, target: LinkType) -> Result<()>;
    async fn remove_link(&self, source: LinkType, target: LinkType) -> Result<()>;
}

impl LinkManagementLocal for PipewireManager {
    async fn create_link(&self, source: LinkType, target: LinkType) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let message = PipewireMessage::CreateDeviceLink(source, target, send);
        self.pipewire().send_message(message)?;
        recv.await?;

        Ok(())
    }

    async fn remove_link(&self, source: LinkType, target: LinkType) -> Result<()> {
        let message = PipewireMessage::RemoveDeviceLink(source, target);
        self.pipewire().send_message(message)
    }
}
