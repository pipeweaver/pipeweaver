use crate::handler::pipewire::components::application::ApplicationManagement;
use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use crate::{APP_ID, APP_NAME};
use anyhow::{Result, anyhow, bail};
use enum_map::{EnumMap, enum_map};
use pipeweaver_pipewire::oneshot;
use pipeweaver_pipewire::{MediaClass, NodeProperties, PipewireMessage};
use pipeweaver_profile::{
    DeviceDescription, PhysicalSourceDevice, PhysicalTargetDevice, VirtualSourceDevice,
    VirtualTargetDevice,
};
use pipeweaver_shared::{Colour, Mix, NodeType, OrderGroup};
use strum::IntoEnumIterator;
use ulid::Ulid;

type GroupList = EnumMap<OrderGroup, Vec<Ulid>>;

/// This crate contains everything needed to create a Pipewire node
pub(crate) trait NodeManagement {
    fn get_node_type(&self, id: Ulid) -> Option<NodeType>;

    async fn node_new(&mut self, node_type: NodeType, name: String) -> Result<Ulid>;

    async fn node_create(
        &mut self,
        node_type: NodeType,
        description: &DeviceDescription,
    ) -> Result<()>;
    async fn node_rename(&mut self, id: Ulid, name: String) -> Result<()>;
    async fn node_remove(&mut self, id: Ulid) -> Result<()>;

    async fn node_set_group(&mut self, id: Ulid, group: OrderGroup) -> Result<()>;
    async fn node_set_position(&mut self, id: Ulid, position: u8) -> Result<()>;

    async fn node_set_colour(&mut self, id: Ulid, colour: Colour) -> Result<()>;
    fn get_target_node_count(&self) -> usize;
}

impl NodeManagement for PipewireManager {
    fn get_node_type(&self, id: Ulid) -> Option<NodeType> {
        let sources = &self.profile.devices.sources;
        if sources
            .physical_devices
            .iter()
            .any(|d| d.description.id == id)
        {
            return Some(NodeType::PhysicalSource);
        }
        if sources
            .virtual_devices
            .iter()
            .any(|d| d.description.id == id)
        {
            return Some(NodeType::VirtualSource);
        }

        let targets = &self.profile.devices.targets;
        if targets
            .physical_devices
            .iter()
            .any(|d| d.description.id == id)
        {
            return Some(NodeType::PhysicalTarget);
        }
        if targets
            .virtual_devices
            .iter()
            .any(|d| d.description.id == id)
        {
            return Some(NodeType::VirtualTarget);
        }
        None
    }

    async fn node_new(&mut self, node_type: NodeType, name: String) -> Result<Ulid> {
        // This is relatively simple, firstly generate the ID, and build the description
        let id = Ulid::new();
        let description = DeviceDescription {
            id,
            name: name.clone(),
            colour: self.get_colour(name),
        };

        // Store this in the profile, and setup default blank routing table
        match node_type {
            NodeType::PhysicalSource => {
                self.profile
                    .devices
                    .sources
                    .physical_devices
                    .push(PhysicalSourceDevice {
                        description: description.clone(),
                        ..Default::default()
                    });
                self.profile.routes.insert(id, Default::default());
                self.profile.devices.sources.device_order[OrderGroup::default()].push(id);
            }
            NodeType::VirtualSource => {
                self.profile
                    .devices
                    .sources
                    .virtual_devices
                    .push(VirtualSourceDevice {
                        description: description.clone(),
                        ..Default::default()
                    });
                self.profile.routes.insert(id, Default::default());
                self.profile.devices.sources.device_order[OrderGroup::default()].push(id);
            }
            NodeType::PhysicalTarget => {
                self.profile
                    .devices
                    .targets
                    .physical_devices
                    .push(PhysicalTargetDevice {
                        description: description.clone(),
                        ..Default::default()
                    });
                self.profile.devices.targets.device_order[OrderGroup::default()].push(id);
            }
            NodeType::VirtualTarget => {
                self.profile
                    .devices
                    .targets
                    .virtual_devices
                    .push(VirtualTargetDevice {
                        description: description.clone(),
                        ..Default::default()
                    });
                self.profile.devices.targets.device_order[OrderGroup::default()].push(id);
            }
        }

        // Create the Nodes and Filters associated with this
        self.node_create(node_type, &description).await?;

        // Load the initial volumes onto the node
        self.load_initial_volume(id).await?;

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

    async fn node_rename(&mut self, id: Ulid, name: String) -> Result<()> {
        // If we're renaming a node, we need to teardown the original node, then rebuild it with
        // the new name. I've checked for an easy way to do this directly in PipeWire, but
        // ultimately couldn't find one.
        //
        // What we need to do here, is teardown the original, update the profile descriptor, then
        // create a new one with the same settings.

        // First thing we need to do, is to find this node
        let err = anyhow!("Unable to find Node");
        let node_type = self.get_node_type(id).ok_or(err)?;

        // Now we remove it, and all associated filters, while leaving it in the profile
        match node_type {
            NodeType::PhysicalSource => self.node_remove_physical_source(id, false).await?,
            NodeType::VirtualSource => self.node_remove_virtual_source(id, false).await?,
            NodeType::PhysicalTarget => self.node_remove_physical_target(id, false).await?,
            NodeType::VirtualTarget => self.node_remove_virtual_target(id, false).await?,
        }

        // Update the name in the profile
        let description = self.get_device_description(id)?;
        description.name = name;

        // Create a local version of this description, create the node tree and load volumes
        let local_desc = description.clone();
        self.node_create(node_type, &local_desc).await?;
        self.load_initial_volume(id).await?;
        self.sync_pipewire_volume(id).await;

        // Re-load the routes
        match node_type {
            NodeType::PhysicalSource | NodeType::VirtualSource => {
                self.routing_load_source(&id).await?;
            }
            NodeType::PhysicalTarget | NodeType::VirtualTarget => {
                self.routing_load_target(&id).await?
            }
        }

        self.refresh_applications(id).await?;
        if node_type == NodeType::PhysicalSource || node_type == NodeType::PhysicalTarget {
            self.connect_for_node(id).await?;
        }

        Ok(())
    }

    async fn node_remove(&mut self, id: Ulid) -> Result<()> {
        // This is complicated, it depends purely on the node type and what we're trying to do here.

        if let Some(node_type) = self.get_node_type(id) {
            match node_type {
                NodeType::PhysicalSource => self.node_remove_physical_source(id, true).await?,
                NodeType::VirtualSource => self.node_remove_virtual_source(id, true).await?,
                NodeType::PhysicalTarget => self.node_remove_physical_target(id, true).await?,
                NodeType::VirtualTarget => self.node_remove_virtual_target(id, true).await?,
            }
        }
        Ok(())
    }

    async fn node_set_group(&mut self, id: Ulid, group: OrderGroup) -> Result<()> {
        let device_order = self.get_device_order_group(id)?;

        // Remove this node from it's existing group
        Self::find_order_group_by_id(id, device_order)?.retain(|d| d != &id);

        // Set it to the front of the new group
        device_order[group].insert(0, id);

        Ok(())
    }

    async fn node_set_position(&mut self, id: Ulid, position: u8) -> Result<()> {
        let device_order = self.get_device_order_group(id)?;
        let order = Self::find_order_group_by_id(id, device_order)?;

        // Remove it from the existing list
        let position = position as usize;
        order.retain(|d| d != &id);
        if position >= order.len() {
            order.push(id);
        } else {
            order.insert(position, id);
        }
        Ok(())
    }

    async fn node_set_colour(&mut self, id: Ulid, colour: Colour) -> Result<()> {
        if let Some(node_type) = self.get_node_type(id) {
            let err = anyhow!("Cannot Find Node");
            match node_type {
                NodeType::PhysicalSource => {
                    self.get_physical_source_mut(id)
                        .ok_or(err)?
                        .description
                        .colour = colour
                }
                NodeType::PhysicalTarget => {
                    self.get_physical_target_mut(id)
                        .ok_or(err)?
                        .description
                        .colour = colour
                }
                NodeType::VirtualSource => {
                    self.get_virtual_source_mut(id)
                        .ok_or(err)?
                        .description
                        .colour = colour
                }
                NodeType::VirtualTarget => {
                    self.get_virtual_target_mut(id)
                        .ok_or(err)?
                        .description
                        .colour = colour
                }
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

    async fn node_remove_physical_source(&mut self, id: Ulid, profile_remove: bool) -> Result<()>;
    async fn node_remove_virtual_source(&mut self, id: Ulid, profile_remove: bool) -> Result<()>;
    async fn node_remove_physical_target(&mut self, id: Ulid, profile_remove: bool) -> Result<()>;
    async fn node_remove_virtual_target(&mut self, id: Ulid, profile_remove: bool) -> Result<()>;
    async fn node_pw_remove(&mut self, id: Ulid) -> Result<()>;

    /// Used to Remove all Links from a Filter
    async fn remove_routes(&mut self, source: Ulid, target: Ulid) -> Result<()>;

    /// Used to set up the parameters needed for a Pipewire Node
    fn create_node_props(&self, class: MediaClass, desc: &DeviceDescription) -> NodeProperties;

    fn get_device_order_group(&mut self, id: Ulid) -> Result<&mut GroupList>;
    fn find_order_group_by_id(id: Ulid, map: &mut GroupList) -> Result<&mut Vec<Ulid>>;
    fn get_colour(&self, name: String) -> Colour;
}

impl NodeManagementLocal for PipewireManager {
    async fn node_create_physical_source(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Physical' source is an audio source that starts with a 'Pass Through' Filter which
        // maps to the Description's ID
        self.filter_pass_create_id(desc.name.clone(), desc.id)
            .await?;

        // Create and attach a meter
        let filter_name = format!("{}-meter", desc.name);
        let meter = self.filter_meter_create(desc.id, filter_name).await?;
        if self.meter_enabled {
            self.link_create_filter_to_filter(desc.id, meter).await?;
        }
        self.meter_map.insert(desc.id, meter);

        let (mix_a, mix_b) = self.node_create_a_b_volumes(desc).await?;

        // Now we need to link our filter to the Mixes
        self.link_create_filter_to_filter(desc.id, mix_a).await?;
        self.link_create_filter_to_filter(desc.id, mix_b).await?;

        // Create a map for this ID to the mixes
        self.source_map
            .insert(desc.id, enum_map! { Mix::A => mix_a, Mix::B => mix_b });

        // And we're done :)
        Ok(())
    }

    async fn node_create_virtual_source(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Virtual' source is a pipewire node that's selectable by the user.
        let properties = self.create_node_props(MediaClass::Sink, desc);
        self.node_pw_create(properties).await?;

        // Create a Meter
        let filter_name = format!("{}-meter", desc.name);
        let meter = self.filter_meter_create(desc.id, filter_name).await?;

        // Attach this to the original source
        if self.meter_enabled {
            self.link_create_node_to_filter(desc.id, meter).await?;
        }
        self.meter_map.insert(desc.id, meter);

        // Generate the A/B Mixes
        let (mix_a, mix_b) = self.node_create_a_b_volumes(desc).await?;

        // Now we need to link our node to the Mixes
        self.link_create_node_to_filter(desc.id, mix_a).await?;
        self.link_create_node_to_filter(desc.id, mix_b).await?;

        // Create a map for this ID to the mixes
        self.source_map
            .insert(desc.id, enum_map! { Mix::A => mix_a, Mix::B => mix_b });

        // And we're done :)
        Ok(())
    }

    async fn node_create_physical_target(&mut self, desc: &DeviceDescription) -> Result<()> {
        // A 'Physical' Target is just a volume filter by itself with the ID of the device
        self.filter_volume_create_id(desc.name.clone(), desc.id)
            .await?;

        let filter_name = format!("{}-meter", desc.name);
        let meter = self.filter_meter_create(desc.id, filter_name).await?;
        if self.meter_enabled {
            self.link_create_filter_to_filter(desc.id, meter).await?;
        }
        self.meter_map.insert(desc.id, meter);

        Ok(())
    }

    async fn node_create_virtual_target(&mut self, desc: &DeviceDescription) -> Result<()> {
        // Virtual Targets (Such as Stream Mix) have a volume node and a target node
        let properties = self.create_node_props(MediaClass::Source, desc);
        self.node_pw_create(properties).await?;

        // Create a meter and attach it to the volume
        let filter_name = format!("{}-meter", desc.name);
        let meter = self.filter_meter_create(desc.id, filter_name).await?;
        if self.meter_enabled {
            self.link_create_node_to_filter(desc.id, meter).await?;
        }
        self.meter_map.insert(desc.id, meter);

        Ok(())
    }

    async fn node_create_a_b_volumes(&mut self, desc: &DeviceDescription) -> Result<(Ulid, Ulid)> {
        let mix_a = self
            .filter_volume_create(format!("{} A", desc.name))
            .await?;
        let mix_b = self
            .filter_volume_create(format!("{} B", desc.name))
            .await?;

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

    async fn node_remove_physical_source(&mut self, id: Ulid, profile_remove: bool) -> Result<()> {
        // So this ID represents the filter attached to one or more physical nodes, so
        // we need to first make sure nothing is connected, and if it is, remove it.
        if let Some(devices) = self.physical_source.get(&id) {
            for device in devices.clone() {
                self.link_remove_unmanaged_to_filter(device, id).await?;
            }
        }

        // Detach and destroy the Meter
        if let Some(&meter) = self.meter_map.get(&id) {
            if self.meter_enabled {
                self.link_remove_filter_to_filter(id, meter).await?;
            }
            self.filter_remove(meter).await?;
            self.meter_map.remove(&id);
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

        // Remove our knowledge of this node inside the General Struct
        self.physical_source.remove(&id);

        if profile_remove {
            // Remove Routing from the Profile Tree
            self.profile.routes.remove(&id);

            let device_order = self.get_device_order_group(id)?;
            Self::find_order_group_by_id(id, device_order)?.retain(|d| d != &id);

            // And finally, remove the Node from the profile tree
            self.profile
                .devices
                .sources
                .physical_devices
                .retain(|device| device.description.id != id);
        }

        Ok(())
    }

    async fn node_remove_virtual_source(&mut self, id: Ulid, profile_remove: bool) -> Result<()> {
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

        // Detach and destroy the Meter
        if let Some(&meter) = self.meter_map.get(&id) {
            if self.meter_enabled {
                self.link_remove_node_to_filter(id, meter).await?;
            }
            self.filter_remove(meter).await?;
            self.meter_map.remove(&id);
        }

        // Remove the Node from the Pipewire tree
        self.node_pw_remove(id).await?;

        // Remove the A/B Filter mapping
        self.source_map.remove(&id);

        if profile_remove {
            // Remove Routing from the Profile Tree
            self.profile.routes.remove(&id);

            // Remove it from the order tree
            let device_order = self.get_device_order_group(id)?;
            Self::find_order_group_by_id(id, device_order)?.retain(|d| d != &id);

            // And finally, remove the Node from the profile tree
            self.profile
                .devices
                .sources
                .virtual_devices
                .retain(|device| device.description.id != id);
        }
        Ok(())
    }

    async fn node_remove_physical_target(&mut self, id: Ulid, profile_remove: bool) -> Result<()> {
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

        // Detach and destroy the Meter
        if let Some(&meter) = self.meter_map.get(&id) {
            if self.meter_enabled {
                self.link_remove_filter_to_filter(id, meter).await?;
            }
            self.filter_remove(meter).await?;
            self.meter_map.remove(&id);
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
        if profile_remove {
            for (_, target) in self.profile.routes.iter_mut() {
                if target.contains(&id) {
                    target.retain(|target_id| target_id != &id);
                }
            }
        }

        // Now we can destroy our 'Volume' filter
        self.filter_remove(id).await?;

        // Remove knowledge of our node
        self.physical_target.remove(&id);

        if profile_remove {
            // Remove from the Order tree
            let device_order = self.get_device_order_group(id)?;
            Self::find_order_group_by_id(id, device_order)?.retain(|d| d != &id);

            // And finally, remove the Node from the profile tree
            self.profile
                .devices
                .targets
                .physical_devices
                .retain(|device| device.description.id != id);
        }

        Ok(())
    }

    async fn node_remove_virtual_target(&mut self, id: Ulid, profile_remove: bool) -> Result<()> {
        // Again, similar to physical targets, but we need to check the target map to
        // find our volume filter then un-route and remove it

        // Detach and destroy the Meter
        if let Some(&meter) = self.meter_map.get(&id) {
            if self.meter_enabled {
                self.link_remove_node_to_filter(id, meter).await?;
            }
            self.filter_remove(meter).await?;
            self.meter_map.remove(&id);
        }

        for (source, targets) in self.profile.routes.clone() {
            if targets.contains(&id) {
                // Grab the A/B Mixes for this source
                if let Some(mix_map) = self.source_map.get(&source) {
                    let mix_map = *mix_map;
                    for mix in Mix::iter() {
                        self.link_remove_filter_to_node(mix_map[mix], id).await?;
                    }
                }
            }
        }

        // Now we can drop the node
        self.node_pw_remove(id).await?;

        if profile_remove {
            // Remove ourselves as a target from any routes we're active in
            self.profile
                .routes
                .iter_mut()
                .for_each(|(_, targets)| targets.retain(|t| *t != id));

            let device_order = self.get_device_order_group(id)?;
            Self::find_order_group_by_id(id, device_order)?.retain(|d| d != &id);

            // Finally remove this node from the profile
            self.profile
                .devices
                .targets
                .virtual_devices
                .retain(|device| device.description.id != id);
        }
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
        let volume = self.get_node_volume(desc.id, Mix::A).unwrap();

        let managed_volume = matches!(class, MediaClass::Sink);

        let identifier = format!("{} {}", APP_NAME, desc.name)
            .to_lowercase()
            .replace(" ", "_");

        NodeProperties {
            node_id: desc.id,
            node_name: identifier.clone(),
            node_nick: identifier,
            node_description: format!("{} {}", APP_NAME, desc.name),
            initial_volume: volume,
            app_id: APP_ID.to_string(),
            app_name: APP_NAME.to_lowercase(),
            linger: false,
            class,
            managed_volume,
            buffer: self.profile.audio_quantum,
            rate: self.clock_rate.unwrap_or(48000),
            ready_sender: None,
        }
    }

    fn get_device_order_group(&mut self, id: Ulid) -> Result<&mut GroupList> {
        if let Some(node_type) = self.get_node_type(id) {
            let device_order = match node_type {
                NodeType::PhysicalSource | NodeType::VirtualSource => {
                    &mut self.profile.devices.sources.device_order
                }
                NodeType::PhysicalTarget | NodeType::VirtualTarget => {
                    &mut self.profile.devices.targets.device_order
                }
            };
            return Ok(device_order);
        }
        bail!("Node Id {} not found", id)
    }

    fn find_order_group_by_id(id: Ulid, map: &mut GroupList) -> Result<&mut Vec<Ulid>> {
        for (_, vec) in map.iter_mut() {
            if vec.contains(&id) {
                return Ok(vec);
            }
        }
        bail!("Id Not Found in Vec List");
    }

    fn get_colour(&self, name: String) -> Colour {
        // This is probably unhelpful in most use cases, but here are some default
        // colours for what people would have as potential default devices.
        match name.as_str() {
            "Microphone" => Colour {
                red: 47,
                green: 24,
                blue: 71,
            },
            "PC Line In" => Colour {
                red: 98,
                green: 17,
                blue: 99,
            },
            "System" => Colour {
                red: 153,
                green: 98,
                blue: 30,
            },
            "Browser" => Colour {
                red: 211,
                green: 139,
                blue: 93,
            },
            "Game" => Colour {
                red: 243,
                green: 255,
                blue: 182,
            },
            "Music" => Colour {
                red: 115,
                green: 158,
                blue: 130,
            },
            "Chat" => Colour {
                red: 44,
                green: 85,
                blue: 48,
            },
            "Headphones" => Colour {
                red: 0,
                green: 255,
                blue: 255,
            },
            "Stream Mix" => Colour {
                red: 19,
                green: 64,
                blue: 116,
            },
            "VOD" => Colour {
                red: 19,
                green: 49,
                blue: 92,
            },
            "Chat Mic" => Colour {
                red: 11,
                green: 37,
                blue: 69,
            },
            _ => Colour {
                red: 0,
                green: 255,
                blue: 255,
            },
        }
    }
}
