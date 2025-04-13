use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME};
use anyhow::{anyhow, bail, Result};
use enum_map::enum_map;
use pipecast_pipewire::{MediaClass, NodeProperties, PipewireMessage};
use pipecast_profile::{DeviceDescription, PhysicalSourceDevice, PhysicalTargetDevice, VirtualSourceDevice, VirtualTargetDevice};
use pipecast_shared::{Colour, Mix, NodeType};
use strum::IntoEnumIterator;
use tokio::sync::oneshot;
use ulid::Ulid;

/// This crate contains everything needed to create a Pipewire node
pub(crate) trait NodeManagement {
    fn get_node_type(&self, id: Ulid) -> Option<NodeType>;
    fn get_target_filter_node(&self, id: Ulid) -> Result<Ulid>;

    async fn node_new(&mut self, node_type: NodeType, name: String) -> Result<Ulid>;

    async fn node_create(&mut self, node_type: NodeType, description: &DeviceDescription) -> Result<()>;
    async fn node_remove(&mut self, id: Ulid) -> Result<()>;

    async fn node_set_colour(&mut self, id: Ulid, colour: Colour) -> Result<()>;
    fn get_target_node_count(&self) -> usize;
}

impl NodeManagement for PipewireManager {
    fn get_node_type(&self, id: Ulid) -> Option<NodeType> {
        let sources = &self.profile.devices.sources;
        if sources.physical_devices.iter().any(|d| d.description.id == id) {
            return Some(NodeType::PhysicalSource);
        }
        if sources.virtual_devices.iter().any(|d| d.description.id == id) {
            return Some(NodeType::VirtualSource);
        }

        let targets = &self.profile.devices.targets;
        if targets.physical_devices.iter().any(|d| d.description.id == id) {
            return Some(NodeType::PhysicalTarget);
        }
        if targets.virtual_devices.iter().any(|d| d.description.id == id) {
            return Some(NodeType::VirtualTarget);
        }
        None
    }

    fn get_target_filter_node(&self, id: Ulid) -> Result<Ulid> {
        let err = anyhow!("Target Node not Found");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        if node_type == NodeType::PhysicalTarget {
            Ok(id)
        } else {
            let err = anyhow!("Unable to Locate Volume for Target");
            Ok(*self.target_map.get(&id).ok_or(err)?)
        }
    }


    async fn node_new(&mut self, node_type: NodeType, name: String) -> Result<Ulid> {
        // This is relatively simple, firstly generate the ID, and build the description
        let id = Ulid::new();
        let description = DeviceDescription {
            id,
            name: name.clone(),
            colour: self.get_colour(name),
        };

        // Create the Nodes and Filters associated with this
        self.node_create(node_type, &description).await?;

        // Store this in the profile, and setup default blank routing table
        match node_type {
            NodeType::PhysicalSource => {
                self.profile.devices.sources.physical_devices.push(PhysicalSourceDevice {
                    description,
                    ..Default::default()
                });
                self.profile.routes.insert(id, Default::default());
            }
            NodeType::VirtualSource => {
                self.profile.devices.sources.virtual_devices.push(VirtualSourceDevice {
                    description,
                    ..Default::default()
                });
                self.profile.routes.insert(id, Default::default());
            }
            NodeType::PhysicalTarget => {
                self.profile.devices.targets.physical_devices.push(PhysicalTargetDevice {
                    description,
                    ..Default::default()
                });
            }
            NodeType::VirtualTarget => {
                self.profile.devices.targets.virtual_devices.push(VirtualTargetDevice {
                    description,
                    ..Default::default()
                });
            }
        }

        Ok(id)
    }

    async fn node_create(&mut self, node_type: NodeType, desc: &DeviceDescription) -> Result<()> {
        // Create the Node or Filter depending on the device
        match node_type {
            NodeType::PhysicalSource => self.node_create_physical_source(desc).await?,
            NodeType::VirtualSource => self.node_create_virtual_source(desc).await?,
            NodeType::PhysicalTarget => self.node_create_physical_target(desc).await?,
            NodeType::VirtualTarget => self.node_create_virtual_target(desc).await?,
        }

        Ok(())
    }


    async fn node_remove(&mut self, id: Ulid) -> Result<()> {
        // This is complicated, it depends purely on the node type and what we're trying to do here.

        if let Some(node_type) = self.get_node_type(id) {
            match node_type {
                NodeType::PhysicalSource => self.node_remove_physical_source(id).await?,
                NodeType::VirtualSource => self.node_remove_virtual_source(id).await?,
                NodeType::PhysicalTarget => self.node_remove_physical_target(id).await?,
                NodeType::VirtualTarget => self.node_remove_virtual_target(id).await?,
            }
        }
        Ok(())
    }

    async fn node_set_colour(&mut self, id: Ulid, colour: Colour) -> Result<()> {
        if let Some(node_type) = self.get_node_type(id) {
            let err = anyhow!("Cannot Find Node");
            match node_type {
                NodeType::PhysicalSource => self.get_physical_source_mut(id).ok_or(err)?.description.colour = colour,
                NodeType::PhysicalTarget => self.get_physical_target_mut(id).ok_or(err)?.description.colour = colour,
                NodeType::VirtualSource => self.get_virtual_source_mut(id).ok_or(err)?.description.colour = colour,
                NodeType::VirtualTarget => self.get_virtual_target_mut(id).ok_or(err)?.description.colour = colour,
            }
        }
        Ok(())
    }

    fn get_target_node_count(&self) -> usize {
        let devices = &self.profile.devices.targets;
        devices.physical_devices.len() + devices.virtual_devices.len()
    }
}

trait NodeManagementLocal {
    /// Used to Create a node inside Pipewire
    async fn node_create_physical_source(&mut self, desc: &DeviceDescription) -> Result<()>;
    async fn node_create_virtual_source(&mut self, desc: &DeviceDescription) -> Result<()>;
    async fn node_create_physical_target(&mut self, desc: &DeviceDescription) -> Result<()>;
    async fn node_create_virtual_target(&mut self, desc: &DeviceDescription) -> Result<()>;
    async fn node_create_a_b_volumes(&mut self, desc: &DeviceDescription) -> Result<(Ulid, Ulid)>;
    async fn node_pw_create(&mut self, props: NodeProperties) -> Result<()>;

    async fn node_remove_physical_source(&mut self, id: Ulid) -> Result<()>;
    async fn node_remove_virtual_source(&mut self, id: Ulid) -> Result<()>;
    async fn node_remove_physical_target(&mut self, id: Ulid) -> Result<()>;
    async fn node_remove_virtual_target(&mut self, id: Ulid) -> Result<()>;
    async fn node_pw_remove(&mut self, id: Ulid) -> Result<()>;


    /// Used to Remove all Links from a Filter
    async fn remove_routes(&mut self, source: Ulid, target: Ulid) -> Result<()>;

    /// Used to set up the parameters needed for a Pipewire Node
    fn create_node_props(&self, class: MediaClass, desc: &DeviceDescription) -> NodeProperties;

    fn get_colour(&self, name: String) -> Colour;
}

impl NodeManagementLocal for PipewireManager {
    async fn node_create_physical_source(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Physical' source is an audio source that starts with a 'Pass Through' Filter which
        // maps to the Description's ID
        self.filter_pass_create_id(desc.name.clone(), desc.id).await?;

        let (mix_a, mix_b) = self.node_create_a_b_volumes(desc).await?;

        // Now we need to link our filter to the Mixes
        self.link_create_filter_to_filter(desc.id, mix_a).await?;
        self.link_create_filter_to_filter(desc.id, mix_b).await?;

        // Create a map for this ID to the mixes
        self.source_map.insert(desc.id, enum_map! { Mix::A => mix_a, Mix::B => mix_b });

        // And we're done :)
        Ok(())
    }

    async fn node_create_virtual_source(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Virtual' source is a pipewire node that's selectable by the user.
        let properties = self.create_node_props(MediaClass::Sink, &desc);
        self.node_pw_create(properties).await?;

        let (mix_a, mix_b) = self.node_create_a_b_volumes(desc).await?;

        // Now we need to link our node to the Mixes
        self.link_create_node_to_filter(desc.id, mix_a).await?;
        self.link_create_node_to_filter(desc.id, mix_b).await?;

        // Create a map for this ID to the mixes
        self.source_map.insert(desc.id, enum_map! { Mix::A => mix_a, Mix::B => mix_b });

        // And we're done :)
        Ok(())
    }

    async fn node_create_physical_target(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Physical' Target is just a volume filter by itself with the ID of the device
        self.filter_volume_create_id(desc.name.clone(), desc.id).await?;

        Ok(())
    }

    async fn node_create_virtual_target(&mut self, desc: &DeviceDescription) -> Result<()> {
        // Virtual Targets (Such as Stream Mix) have a volume node and a target node
        let properties = self.create_node_props(MediaClass::Source, &desc);
        self.node_pw_create(properties).await?;

        // Link the Volume to the Target Node
        let volume = self.filter_volume_create(desc.name.clone()).await?;
        self.link_create_filter_to_node(volume, desc.id).await?;

        // Map this Node to this Volume
        self.target_map.insert(desc.id, volume);

        Ok(())
    }

    async fn node_create_a_b_volumes(&mut self, desc: &DeviceDescription) -> Result<(Ulid, Ulid)> {
        let mix_a = self.filter_volume_create(format!("{} A", desc.name)).await?;
        let mix_b = self.filter_volume_create(format!("{} B", desc.name)).await?;

        Ok((mix_a, mix_b))
    }

    async fn node_pw_create(&mut self, mut props: NodeProperties) -> Result<()> {
        let (send, recv) = oneshot::channel();
        props.ready_sender = Some(send);

        let message = PipewireMessage::CreateDeviceNode(props);
        self.pipewire().send_message(message)?;
        recv.await?;

        Ok(())
    }

    async fn node_remove_physical_source(&mut self, id: Ulid) -> Result<()> {
        // So this ID represents the filter attached to one or more physical nodes, so
        // we need to first make sure nothing is connected, and if it is, remove it.
        if let Some(devices) = self.physical_source.get(&id) {
            // TODO: Wanker Check
            for device in devices.clone() {
                self.link_remove_unmanaged_to_filter(device, id).await?;
            }
        }

        // Next, we detach the links from the pass through to the A/B mixes
        if let Some(mix_map) = self.source_map.get(&id) {
            let mix_map = *mix_map;
            for mix in Mix::iter() {
                self.link_remove_filter_to_filter(id, mix_map[mix]).await?;

                // Remove all links from this Mix to all defined outputs
                self.remove_routes(id, mix_map[mix]).await?;

                // Should be fully detached, remove the Mix filter
                self.filter_remove(mix_map[mix]).await?
            }
        }

        // Remove the Base pass through filter from the tree
        self.filter_remove(id).await?;

        // Remove the A/B Filter mapping
        self.source_map.remove(&id);

        // Remove Routing from the Profile Tree
        self.profile.routes.remove(&id);

        // Remove our knowledge of this node inside the General Struct
        self.physical_source.remove(&id);

        // And finally, remove the Node from the profile tree
        self.profile.devices.sources.physical_devices.retain(|device| device.description.id != id);

        Ok(())
    }

    async fn node_remove_virtual_source(&mut self, id: Ulid) -> Result<()> {
        // Virtual Sources are a little easier, still a bit of a repeat from the above
        // in places, but we don't have to deal with Unmanaged sources, and our node
        // connects directly to the Mix A / B volume filters
        if let Some(mix_map) = self.source_map.get(&id) {
            let mix_map = *mix_map;
            for mix in Mix::iter() {
                self.link_remove_node_to_filter(id, mix_map[mix]).await?;

                // Remove all links from this Mix to all defined outputs
                self.remove_routes(id, mix_map[mix]).await?;

                // Should be fully detached, remove the Mix filter
                self.filter_remove(mix_map[mix]).await?
            }
        }

        // Remove the Node from the Pipewire tree
        self.node_pw_remove(id).await?;

        // Remove the A/B Filter mapping
        self.source_map.remove(&id);

        // Remove Routing from the Profile Tree
        self.profile.routes.remove(&id);

        // And finally, remove the Node from the profile tree
        self.profile.devices.sources.virtual_devices.retain(|device| device.description.id != id);

        Ok(())
    }

    async fn node_remove_physical_target(&mut self, id: Ulid) -> Result<()> {
        // These are kinda similar to PhysicalSources, except we're looking in the other
        // direction (Filter -> Device)
        // So this ID represents the filter attached to one or more physical nodes, so
        // we need to first make sure nothing is connected, and if it is, remove it.
        if let Some(devices) = self.physical_target.get(&id) {
            // Detach from the Volume Filter to the Physical Node
            for device in devices.clone() {
                self.link_remove_filter_to_unmanaged(id, device).await?;
            }
        }

        // Next, we need to detach anything that may be routing to us
        for (source, targets) in self.profile.routes.clone() {
            // Are we a target for this route?
            if targets.contains(&id) {
                // Pull out the Mixes for this source
                if let Some(mix_map) = self.source_map.get(&source) {
                    let mix_map = *mix_map;
                    // Drop our Link on All Mixes
                    for mix in Mix::iter() {
                        // Flag this for removal, this gets done slightly later
                        self.link_remove_filter_to_filter(mix_map[mix], id).await?;
                    }
                }
            }
        }

        // We'll re-iterate the routes and make sure our node is removed from the Profile
        for (_, target) in self.profile.routes.iter_mut() {
            if target.contains(&id) {
                target.retain(|target_id| target_id != &id);
            }
        }

        // Now we can destroy our 'Volume' filter
        self.filter_remove(id).await?;

        // Remove knowledge of our node
        self.physical_target.remove(&id);

        // And finally, remove the Node from the profile tree
        self.profile.devices.targets.physical_devices.retain(|device| device.description.id != id);

        Ok(())
    }

    async fn node_remove_virtual_target(&mut self, id: Ulid) -> Result<()> {
        // Again, similar to physical targets, but we need to check the target map to
        // find our volume filter then un-route and remove it
        if let Some(volume) = self.target_map.clone().get(&id) {
            // Detach from the Volume Filter to the Target Node
            self.link_remove_filter_to_node(*volume, id).await?;

            for (source, targets) in self.profile.routes.clone() {
                if targets.contains(&id) {
                    // Grab the A/B Mixes for this source
                    if let Some(mix_map) = self.source_map.get(&source) {
                        let mix_map = *mix_map;
                        for mix in Mix::iter() {
                            self.link_remove_filter_to_filter(mix_map[mix], *volume).await?;
                        }
                    }
                }
            }

            // Volume Filter should be clean now, remove it
            self.filter_remove(*volume).await?;
        }

        // Now we can drop the node
        self.node_pw_remove(id).await?;

        // Remove it from the target map
        self.target_map.remove(&id);

        // Remove ourselves as a target from any routes we're active in
        self.profile.routes.iter_mut().for_each(|(_, targets)| targets.retain(|t| *t != id));

        // Finally remove this node from the profile
        self.profile.devices.targets.virtual_devices.retain(|device| device.description.id != id);

        Ok(())
    }

    async fn node_pw_remove(&mut self, id: Ulid) -> Result<()> {
        let message = PipewireMessage::RemoveDeviceNode(id);
        self.pipewire().send_message(message)?;
        Ok(())
    }

    async fn remove_routes(&mut self, source: Ulid, target: Ulid) -> Result<()> {
        if let Some(route) = self.profile.routes.get(&source) {
            let route = route.clone();
            for route in route {
                self.link_remove_filter_to_filter(target, route).await?;
            }
        }
        Ok(())
    }

    fn create_node_props(&self, class: MediaClass, desc: &DeviceDescription) -> NodeProperties {
        let identifier = format!("{} {}", APP_NAME, desc.name)
            .to_lowercase()
            .replace(" ", "_");

        NodeProperties {
            node_id: desc.id,
            node_name: identifier.clone(),
            node_nick: identifier,
            node_description: format!("{} {}", APP_NAME, desc.name),
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_lowercase(),
            linger: false,
            class,
            ready_sender: None,
        }
    }

    fn get_colour(&self, name: String) -> Colour {
        // This is probably unhelpful in most use cases, but here are some default
        // colours for what people would have as potential default devices.
        match name.as_str() {
            "Microphone" => Colour { red: 47, green: 24, blue: 71 },
            "PC Line In" => Colour { red: 98, green: 17, blue: 99 },
            "System" => Colour { red: 153, green: 98, blue: 30 },
            "Browser" => Colour { red: 211, green: 139, blue: 93 },
            "Game" => Colour { red: 243, green: 255, blue: 182 },
            "Music" => Colour { red: 115, green: 158, blue: 130 },
            "Chat" => Colour { red: 44, green: 85, blue: 48 },
            "Headphones" => Colour { red: 0, green: 255, blue: 255 },
            "Stream Mix" => Colour { red: 19, green: 64, blue: 116 },
            "VOD" => Colour { red: 19, green: 49, blue: 92 },
            "Chat Mic" => Colour { red: 11, green: 37, blue: 69 },
            _ => Colour { red: 0, green: 255, blue: 255 }
        }
    }
}