use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use log::debug;
use pipecast_ipc::commands::PhysicalDevice;
use pipecast_profile::{PhysicalDeviceDescriptor, PhysicalSourceDevice};
use pipecast_shared::{DeviceType, NodeType};
use ulid::Ulid;

pub(crate) trait PhysicalDevices {
    async fn source_device_added(&mut self, node: PhysicalDevice) -> Result<()>;
    async fn target_device_added(&mut self, node: PhysicalDevice) -> Result<()>;

    async fn source_device_removed(&mut self, node_id: u32) -> Result<()>;
    async fn target_device_removed(&mut self, node_id: u32) -> Result<()>;

    async fn add_device_to_node(&mut self, id: Ulid, node_id: u32) -> Result<()>;
    async fn remove_device_from_node(&mut self, id: Ulid, vec_index: usize) -> Result<()>;
}

impl PhysicalDevices for PipewireManager {
    async fn source_device_added(&mut self, node: PhysicalDevice) -> Result<()> {
        self.node_list[DeviceType::Source].push(node.clone());

        // We need to check through our profile to see if we can find this device
        let devices = self.profile.devices.sources.physical_devices.clone();
        'start: for (dev_i, device) in devices.iter().enumerate()
        {
            // Clarification needed first, we go over the 'attached device' list twice, the
            // first time to match the absolute ALSA defined name, and the second time to
            // attempt to match the description (Human Readable Name). The main reason we do
            // this is so if you have two devices with the same description, we don't want to
            // incorrectly match, or consider the work done, the ALSA match should hopefully catch
            // them all.

            if let Some(node_name) = &node.name {
                for (name_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(name) = &dev.name {
                        if name == node_name {
                            debug!("Attaching Node {} to {}", node.id, device.description.id);

                            // Got a hit, attach to our filter, and bring it into the tree
                            self.link_create_unmanaged_to_filter(node.id, device.description.id)
                                .await?;

                            // We'll force upgrade the description regardless, just to ensure the
                            // node is accurately represented
                            let mut descriptor = dev.clone();
                            descriptor.description = node.description.clone();

                            let mut device = device.clone();
                            device.attached_devices[name_i] = descriptor;
                            self.profile.devices.sources.physical_devices[dev_i] = device;

                            break 'start;
                        }
                    }
                }
            }

            if let Some(node_desc) = &node.description {
                for (desc_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(desc) = &dev.description {
                        if desc == node_desc {
                            // Firstly, attach the Node
                            debug!("Attaching Node {} to {}", node.id, device.description.id);
                            self.link_create_unmanaged_to_filter(node.id, device.description.id)
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

                            break 'start;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn target_device_added(&mut self, node: PhysicalDevice) -> Result<()> {
        self.node_list[DeviceType::Target].push(node.clone());

        // Same as source node above, so read the comments there :)
        let devices = self.profile.devices.targets.physical_devices.clone();
        'start: for (dev_i, device) in devices.iter().enumerate()
        {
            if let Some(node_name) = &node.name {
                for (name_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(name) = &dev.name {
                        if name == node_name {
                            debug!("Attaching Node {} to {}", node.id, device.description.id);

                            // Got a hit, attach to our filter, and bring it into the tree
                            self.link_create_filter_to_unmanaged(device.description.id, node.id).await?;

                            let mut descriptor = dev.clone();
                            descriptor.description = node.description.clone();

                            let mut device = device.clone();
                            device.attached_devices[name_i] = descriptor;
                            self.profile.devices.targets.physical_devices[dev_i] = device;
                            break 'start;
                        }
                    }
                }
            }

            if let Some(node_desc) = &node.description {
                for (desc_i, dev) in device.attached_devices.iter().enumerate() {
                    if let Some(desc) = &dev.description {
                        if desc == node_desc {
                            // Firstly, attach the Node
                            debug!("Attaching Node {} to {}", device.description.id, node.id);
                            self.link_create_filter_to_unmanaged(device.description.id, node.id).await?;

                            debug!("Updating Profile Node to Name: {:?}", node.name);
                            let mut descriptor = dev.clone();
                            descriptor.name = node.name.clone();

                            let mut device = device.clone();
                            device.attached_devices[desc_i] = descriptor;
                            self.profile.devices.targets.physical_devices[dev_i] = device;

                            break 'start;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn source_device_removed(&mut self, node_id: u32) -> Result<()> {
        self.node_list[DeviceType::Source].retain(|node| node.id != node_id);
        Ok(())
    }

    async fn target_device_removed(&mut self, node_id: u32) -> Result<()> {
        self.node_list[DeviceType::Target].retain(|node| node.id != node_id);
        Ok(())
    }

    async fn add_device_to_node(&mut self, id: Ulid, node_id: u32) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        let error = anyhow!("Unable to Locate Node: {}", id);
        let pw_error = anyhow!("Unable to locate Pipewire Node: {}", node_id);

        // Find the Pipewire Node
        let node = self.physical_nodes.iter().find(|node| node.node_id == node_id).ok_or(pw_error)?.clone();

        match node_type {
            NodeType::PhysicalSource => {
                let device = self.get_physical_source_mut(id).ok_or(error)?;

                let new_node = PhysicalDeviceDescriptor {
                    name: node.name.clone(),
                    description: node.description.clone(),
                };

                device.attached_devices.push(new_node);
                // TODO: Connect this node
            }
            NodeType::PhysicalTarget => {
                let device = self.get_physical_target_mut(id).ok_or(error)?;

                let new_node = PhysicalDeviceDescriptor {
                    name: node.name.clone(),
                    description: node.description.clone(),
                };

                device.attached_devices.push(new_node);
                // TODO: Connect this node
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
                device.attached_devices.remove(vec_index);

                // TODO: Detach this Node
            }
            NodeType::PhysicalTarget => {
                let device = self.get_physical_target_mut(id).ok_or(error)?;
                device.attached_devices.remove(vec_index);

                // TODO: Detach this Node
            }
            _ => bail!("Node is not a Physical Node")
        }

        Ok(())
    }
}
