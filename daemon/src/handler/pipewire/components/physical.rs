use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use crate::handler::primary_worker::WorkerMessage;
use anyhow::{Result, anyhow, bail};
use log::debug;
use pipeweaver_ipc::commands::PhysicalDevice;
use pipeweaver_pipewire::DeviceNode;
use pipeweaver_profile::PhysicalDeviceDescriptor;
use pipeweaver_shared::{DeviceType, NodeType};
use tokio::sync::mpsc::Sender;
use ulid::Ulid;

// TODO: This file *REALLY* needs some work :D
pub(crate) trait PhysicalDevices {
    async fn connect_for_node(&mut self, id: Ulid) -> Result<()>;

    async fn source_device_added(
        &mut self,
        node: PhysicalDevice,
        sender: Sender<WorkerMessage>,
    ) -> Result<()>;
    async fn target_device_added(
        &mut self,
        node: PhysicalDevice,
        sender: Sender<WorkerMessage>,
    ) -> Result<()>;

    async fn source_device_removed(&mut self, node_id: u32) -> Result<()>;
    async fn target_device_removed(&mut self, node_id: u32) -> Result<()>;

    async fn source_device_disconnect(&mut self, node_id: u32) -> Result<()>;
    async fn target_device_disconnect(&mut self, node_id: u32) -> Result<()>;

    async fn add_device_to_node(&mut self, id: Ulid, node_id: u32) -> Result<()>;
    async fn remove_device_from_node(&mut self, id: Ulid, vec_index: usize) -> Result<()>;
}

impl PhysicalDevices for PipewireManager {
    async fn connect_for_node(&mut self, id: Ulid) -> Result<()> {
        let err = anyhow!("Cannot Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(
            node_type,
            NodeType::PhysicalTarget | NodeType::PhysicalSource
        ) {
            bail!("Provided Target is not a Physical Node");
        }

        let err = anyhow!("Cannot Find Target Node by ID: {}", id);
        let devices = match node_type {
            NodeType::PhysicalSource => {
                let node = self
                    .profile
                    .devices
                    .sources
                    .physical_devices
                    .iter()
                    .find(|node| node.description.id == id)
                    .ok_or(err)?;
                node.attached_devices.clone()
            }
            NodeType::PhysicalTarget => {
                let node = self
                    .profile
                    .devices
                    .targets
                    .physical_devices
                    .iter()
                    .find(|node| node.description.id == id)
                    .ok_or(err)?;
                node.attached_devices.clone()
            }
            _ => {
                bail!("Incorrect Node Type");
            }
        };

        // While during 'device added' we attempt to fix the profile, we're not going to do that
        // here, as we're iterating over known configuration, so we'll avoid the 'best guess'
        // description checks, the node Names should be valid.
        match node_type {
            NodeType::PhysicalSource => {
                for device in &self.node_list[DeviceType::Source] {
                    if !device.is_usable {
                        continue;
                    }

                    // Try and match this against our node, check by Name first
                    for paired in &devices {
                        // Check by Name First
                        if paired.name == device.name {
                            self.link_create_unmanaged_to_filter(device.node_id, id)
                                .await?;
                        }
                    }
                }
            }
            NodeType::PhysicalTarget => {
                for device in &self.node_list[DeviceType::Target] {
                    if !device.is_usable {
                        continue;
                    }

                    // Try and match this against our node, check by Name first
                    for paired in &devices {
                        // Check by Name First
                        if paired.name == device.name {
                            self.link_create_filter_to_unmanaged(id, device.node_id)
                                .await?;
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn source_device_added(
        &mut self,
        node: PhysicalDevice,
        sender: Sender<WorkerMessage>,
    ) -> Result<()> {
        // We need to check through our profile to see if we can find this device
        let devices = self.profile.devices.sources.physical_devices.clone();
        'start: for (dev_i, device) in devices.iter().enumerate() {
            // Clarification needed first, we go over the 'attached device' list twice, the
            // first time to match the absolute ALSA defined name, and the second time to
            // attempt to match the description (Human Readable Name). The main reason we do
            // this is so if you have two devices with the same description, we don't want to
            // incorrectly match, or consider the work done, the ALSA match should hopefully catch
            // them all.

            if let Some(node_name) = &node.name {
                for (name_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(name) = &dev.name
                        && name == node_name
                    {
                        debug!(
                            "Attaching Node {} to {}",
                            node.node_id, device.description.id
                        );

                        // Got a hit, attach to our filter, and bring it into the tree
                        self.link_create_unmanaged_to_filter(node.node_id, device.description.id)
                            .await?;

                        // We'll force upgrade the description regardless, just to ensure the
                        // node is accurately represented
                        let mut descriptor = dev.clone();
                        descriptor.description = node.description.clone();

                        let mut device = device.clone();
                        device.attached_devices[name_i] = descriptor;
                        self.profile.devices.sources.physical_devices[dev_i] = device;

                        // Let the Primary Worker know we've changed the config
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }

            if let Some(node_desc) = &node.description {
                for (desc_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(desc) = &dev.description
                        && desc == node_desc
                    {
                        // Firstly, attach the Node
                        debug!(
                            "Attaching Node {} to {}",
                            node.node_id, device.description.id
                        );
                        self.link_create_unmanaged_to_filter(node.node_id, device.description.id)
                            .await?;

                        debug!("Updating Profile Node to Name: {:?}", node.name);

                        // This is kinda ugly, but due to likely a changed node name, we need to
                        // update the profile to ensure this now matches the new location for
                        // future checks. Again, we *WANT* to defer to the name where possible.
                        let mut descriptor = dev.clone();
                        descriptor.name = node.name.clone();

                        let mut device = device.clone();
                        device.attached_devices[desc_i] = descriptor;
                        self.profile.devices.sources.physical_devices[dev_i] = device;
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }
        }

        Ok(())
    }

    async fn target_device_added(
        &mut self,
        node: PhysicalDevice,
        sender: Sender<WorkerMessage>,
    ) -> Result<()> {
        // Same as source node above, so read the comments there :)
        let devices = self.profile.devices.targets.physical_devices.clone();
        'start: for (dev_i, device) in devices.iter().enumerate() {
            if let Some(node_name) = &node.name {
                for (name_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(name) = &dev.name
                        && name == node_name
                    {
                        debug!(
                            "Attaching Node {} to {}",
                            node.node_id, device.description.id
                        );

                        // Got a hit, attach to our filter, and bring it into the tree
                        self.link_create_filter_to_unmanaged(device.description.id, node.node_id)
                            .await?;

                        let mut descriptor = dev.clone();
                        descriptor.description = node.description.clone();

                        let mut device = device.clone();
                        device.attached_devices[name_i] = descriptor;
                        self.profile.devices.targets.physical_devices[dev_i] = device;

                        // Let the Primary Worker know we've changed the config
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }

            if let Some(node_desc) = &node.description {
                for (desc_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(desc) = &dev.description
                        && desc == node_desc
                    {
                        // Firstly, attach the Node
                        debug!(
                            "Attaching Node {} to {}",
                            device.description.id, node.node_id
                        );
                        self.link_create_filter_to_unmanaged(device.description.id, node.node_id)
                            .await?;

                        debug!("Updating Profile Node to Name: {:?}", node.name);
                        let mut descriptor = dev.clone();
                        descriptor.name = node.name.clone();

                        let mut device = device.clone();
                        device.attached_devices[desc_i] = descriptor;
                        self.profile.devices.targets.physical_devices[dev_i] = device;

                        // Let the Primary Worker know we've changed the config
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }
        }

        let devices = self.profile.devices.targets.virtual_devices.clone();
        'start: for (dev_i, device) in devices.iter().enumerate() {
            if let Some(node_name) = &node.name {
                for (name_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(name) = &dev.name
                        && name == node_name
                    {
                        debug!(
                            "Attaching Node {} to {}",
                            node.node_id, device.description.id
                        );

                        // Got a hit, attach to our filter, and bring it into the tree
                        self.link_create_node_to_unmanaged(device.description.id, node.node_id)
                            .await?;

                        let mut descriptor = dev.clone();
                        descriptor.description = node.description.clone();

                        let mut device = device.clone();
                        device.attached_devices[name_i] = descriptor;
                        self.profile.devices.targets.virtual_devices[dev_i] = device;

                        // Let the Primary Worker know we've changed the config
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }

            if let Some(node_desc) = &node.description {
                for (desc_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(desc) = &dev.description
                        && desc == node_desc
                    {
                        // Firstly, attach the Node
                        debug!(
                            "Attaching Node {} to {}",
                            device.description.id, node.node_id
                        );
                        self.link_create_node_to_unmanaged(device.description.id, node.node_id)
                            .await?;

                        debug!("Updating Profile Node to Name: {:?}", node.name);
                        let mut descriptor = dev.clone();
                        descriptor.name = node.name.clone();

                        let mut device = device.clone();
                        device.attached_devices[desc_i] = descriptor;
                        self.profile.devices.targets.virtual_devices[dev_i] = device;

                        // Let the Primary Worker know we've changed the config
                        let _ = sender.send(WorkerMessage::ProfileChanged).await;

                        break 'start;
                    }
                }
            }
        }

        Ok(())
    }

    async fn source_device_removed(&mut self, node_id: u32) -> Result<()> {
        self.node_list[DeviceType::Source].retain(|node| node.node_id != node_id);
        Ok(())
    }

    async fn target_device_removed(&mut self, node_id: u32) -> Result<()> {
        self.node_list[DeviceType::Target].retain(|node| node.node_id != node_id);
        Ok(())
    }

    /// Disconnect a source device from all connected filters without removing it from tracking
    async fn source_device_disconnect(&mut self, node_id: u32) -> Result<()> {
        // Search through all physical source devices in the profile to find connections
        let devices = self.profile.devices.sources.physical_devices.clone();
        for device in devices {
            // Check each attached device to see if it matches this node_id
            for attached in &device.attached_devices {
                // Try to locate this attached device in our device_nodes
                if let Some(pw_node) = self.locate_node(attached.clone())
                    && pw_node.node_id == node_id
                {
                    // Found it! Remove the link from this unmanaged node to the filter
                    debug!(
                        "Disconnecting Source Node {} from Filter {}",
                        node_id, device.description.id
                    );
                    let _ = self
                        .link_remove_unmanaged_to_filter(node_id, device.description.id)
                        .await;
                }
            }
        }
        Ok(())
    }

    /// Disconnect a target device from all connected filters without removing it from tracking
    async fn target_device_disconnect(&mut self, node_id: u32) -> Result<()> {
        // Search through physical target devices
        let physical_devices = self.profile.devices.targets.physical_devices.clone();
        for device in physical_devices {
            for attached in &device.attached_devices {
                if let Some(pw_node) = self.locate_node(attached.clone())
                    && pw_node.node_id == node_id
                {
                    debug!(
                        "Disconnecting Target Node {} from Filter {}",
                        node_id, device.description.id
                    );
                    let _ = self
                        .link_remove_filter_to_unmanaged(device.description.id, node_id)
                        .await;
                }
            }
        }

        // Search through virtual target devices
        let virtual_devices = self.profile.devices.targets.virtual_devices.clone();
        for device in virtual_devices {
            for attached in &device.attached_devices {
                if let Some(pw_node) = self.locate_node(attached.clone())
                    && pw_node.node_id == node_id
                {
                    debug!(
                        "Disconnecting Target Node {} from Virtual Node {}",
                        node_id, device.description.id
                    );
                    let _ = self
                        .link_remove_node_to_unmanaged(device.description.id, node_id)
                        .await;
                }
            }
        }
        Ok(())
    }

    async fn add_device_to_node(&mut self, id: Ulid, node_id: u32) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        let error = anyhow!("Unable to Locate Node: {}", id);
        let pw_error = anyhow!("Unable to locate Pipewire Node: {}", node_id);

        // Find the Pipewire Node
        let node = self.device_nodes.get(&node_id).ok_or(pw_error)?.clone();
        if !node.is_usable {
            bail!("Pipewire Node is not usable");
        }

        match node_type {
            NodeType::PhysicalSource => {
                let device = self.get_physical_source_mut(id).ok_or(error)?;

                let new_node = PhysicalDeviceDescriptor {
                    name: node.name.clone(),
                    description: node.description.clone(),
                };

                device.attached_devices.push(new_node.clone());
                let pw_node = self.locate_node(new_node);
                if let Some(node) = pw_node {
                    self.link_create_unmanaged_to_filter(node.node_id, id)
                        .await?;
                }
            }
            NodeType::PhysicalTarget => {
                let device = self.get_physical_target_mut(id).ok_or(error)?;

                let new_node = PhysicalDeviceDescriptor {
                    name: node.name.clone(),
                    description: node.description.clone(),
                };

                device.attached_devices.push(new_node.clone());
                let pw_node = self.locate_node(new_node);
                if let Some(node) = pw_node {
                    self.link_create_filter_to_unmanaged(id, node.node_id)
                        .await?;
                }
            }
            NodeType::VirtualTarget => {
                let device = self.get_virtual_target_mut(id).ok_or(error)?;

                let new_node = PhysicalDeviceDescriptor {
                    name: node.name.clone(),
                    description: node.description.clone(),
                };

                device.attached_devices.push(new_node.clone());
                let pw_node = self.locate_node(new_node);
                if let Some(node) = pw_node {
                    self.link_create_node_to_unmanaged(id, node.node_id).await?;
                }
            }
            _ => bail!("Node is not a Physical Node"),
        }

        Ok(())
    }

    async fn remove_device_from_node(&mut self, id: Ulid, vec_index: usize) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        let error = anyhow!("Unable to Locate Node: {}", id);

        match node_type {
            NodeType::PhysicalSource => {
                let device = self.get_physical_source_mut(id).ok_or(error)?;
                let descriptor = device.attached_devices.remove(vec_index);

                // Attempt to locate this node in our list
                let pw_node = self.locate_node(descriptor);
                if let Some(node) = pw_node {
                    self.link_remove_unmanaged_to_filter(node.node_id, id)
                        .await?;
                }
            }
            NodeType::PhysicalTarget => {
                let device = self.get_physical_target_mut(id).ok_or(error)?;
                let descriptor = device.attached_devices.remove(vec_index);

                // Attempt to locate this node in our list
                let pw_node = self.locate_node(descriptor);
                if let Some(node) = pw_node {
                    self.link_remove_filter_to_unmanaged(id, node.node_id)
                        .await?;
                }
            }
            NodeType::VirtualTarget => {
                debug!("Removing From Virtual Target?");
                let device = self.get_virtual_target_mut(id).ok_or(error)?;
                let descriptor = device.attached_devices.remove(vec_index);

                // Attempt to locate this node in our list
                let pw_node = self.locate_node(descriptor);
                if let Some(node) = pw_node {
                    self.link_remove_node_to_unmanaged(id, node.node_id).await?;
                }
            }
            _ => bail!("Node is not a Physical Node"),
        }

        Ok(())
    }
}

trait PhysicalDevicesLocal {
    fn locate_node(&self, descriptor: PhysicalDeviceDescriptor) -> Option<&DeviceNode>;
}

impl PhysicalDevicesLocal for PipewireManager {
    fn locate_node(&self, descriptor: PhysicalDeviceDescriptor) -> Option<&DeviceNode> {
        if let Some(name) = descriptor.name {
            let node = self
                .device_nodes
                .values()
                .find(|node| node.name.as_ref() == Some(&name));
            if let Some(node) = node {
                return Some(node);
            }
        }

        if let Some(desc) = descriptor.description {
            return self
                .device_nodes
                .values()
                .find(|node| node.description.as_ref() == Some(&desc));
        }

        None
    }
}
