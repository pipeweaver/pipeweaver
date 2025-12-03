use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{bail, Result};
use log::{debug, warn};
use pipeweaver_pipewire::PipewireMessage::{
    ClearApplicationTarget, SetApplicationMute, SetApplicationTarget, SetApplicationVolume,
};
use pipeweaver_pipewire::{ApplicationNode, MediaClass, NodeTarget};
use pipeweaver_shared::{AppDefinition, DeviceType, NodeType};
use std::collections::HashMap;
use ulid::Ulid;

type Target = Option<Option<NodeTarget>>;
pub(crate) trait ApplicationManagement {
    async fn set_application_target(&mut self, def: AppDefinition, target: Ulid) -> Result<()>;
    async fn clear_application_target(&mut self, def: AppDefinition) -> Result<()>;
    async fn set_application_transient_target(&mut self, id: u32, target: Ulid) -> Result<()>;
    async fn clear_application_transient_target(&mut self, id: u32) -> Result<()>;
    async fn set_application_volume(&mut self, id: u32, volume: u8) -> Result<()>;
    async fn set_application_mute(&mut self, id: u32, mute: bool) -> Result<()>;

    fn application_appeared(&mut self, node: ApplicationNode) -> Result<()>;
    fn application_target_changed(&mut self, id: u32, target: Target) -> Result<()>;
    fn application_volume_changed(&mut self, id: u32, volume: u8) -> Result<()>;
    fn application_title_changed(&mut self, id: u32, title: String) -> Result<()>;
    fn application_mute_changed(&mut self, id: u32, muted: bool) -> Result<()>;
    fn application_removed(&mut self, id: u32) -> Result<()>;
}

impl ApplicationManagement for PipewireManager {
    async fn set_application_target(&mut self, def: AppDefinition, target: Ulid) -> Result<()> {
        // Perform Validation on the Target
        if let Some(target) = self.get_application_type_from_node(target) {
            if target != def.device_type {
                bail!("Device Type Mismatch");
            }
        } else {
            bail!("Target not found: {}", target);
        }

        // Ok, first, does this binary exist in the profile?
        let map = &mut self.profile.application_mapping[def.device_type];

        // Update or Insert the definition into the map
        let process_name = def.process.clone();
        let app_name = def.name.clone();

        if let Some(app) = map.get_mut(&process_name) {
            app.insert(app_name, target);
        } else {
            map.insert(process_name, HashMap::from([(app_name, target)]));
        }

        // Next, we need to find all nodes which match this definition
        let matching_nodes = self.find_matching_nodes(&def);

        // Send all the nodes across to the new target
        for node in matching_nodes {
            let message = SetApplicationTarget(node, target);
            self.pipewire().send_message(message)?;
        }

        Ok(())
    }

    async fn clear_application_target(&mut self, def: AppDefinition) -> Result<()> {
        let map = &mut self.profile.application_mapping[def.device_type];
        if let Some(app) = map.get_mut(&def.process) {
            app.remove(&def.name);

            // If there are no apps left, remove the process
            if app.is_empty() {
                map.remove(&def.process);
            }
        }

        let matching_nodes = self.find_matching_nodes(&def);
        for node in matching_nodes {
            let message = ClearApplicationTarget(node);
            self.pipewire().send_message(message)?;
        }

        Ok(())
    }

    async fn set_application_transient_target(&mut self, id: u32, target: Ulid) -> Result<()> {
        if let Some(node) = self.application_nodes.get(&id) {
            if let Some(target) = self.get_application_type_from_node(target) {
                if target != get_application_type(node.node_class) {
                    bail!("Target Type mismatch");
                }
            } else {
                bail!("Invalid Target");
            }

            // Send this node to its new target
            let message = SetApplicationTarget(id, target);
            self.pipewire().send_message(message)?;
        }

        Ok(())
    }

    async fn clear_application_transient_target(&mut self, id: u32) -> Result<()> {
        // We need to force this transient target back to the default output
        if self.application_nodes.contains_key(&id) {
            let message = ClearApplicationTarget(id);
            self.pipewire().send_message(message)?;
        }
        Ok(())
    }

    async fn set_application_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        if !self.application_nodes.contains_key(&id) {
            bail!("Invalid Application Specified");
        }
        if !(0..=100).contains(&volume) {
            bail!("Volume out of range");
        }
        let message = SetApplicationVolume(id, volume);
        self.pipewire().send_message(message)?;
        Ok(())
    }

    async fn set_application_mute(&mut self, id: u32, mute: bool) -> Result<()> {
        if !self.application_nodes.contains_key(&id) {
            bail!("Invalid Application Specified");
        }
        let message = SetApplicationMute(id, mute);
        self.pipewire().send_message(message)?;
        Ok(())
    }

    fn application_appeared(&mut self, node: ApplicationNode) -> Result<()> {
        debug!("Node Appeared: {:?}", node);

        // Get the current node id, and it's reported target
        let node_id = node.node_id;
        let node_target = node.media_target;

        // Add this to our node list
        self.application_nodes.insert(node_id, node);

        if self.application_target_ignore.contains(&node_id) {
            debug!("Application node is ignored, we're done here.");
            return Ok(());
        }

        if let Some(target) = self.get_application_assignment(node_id) {
            let message = SetApplicationTarget(node_id, target);

            // Compare against the nodes arrival target
            match node_target {
                None => {
                    // We've not received a RouteTarget for this app, so redirect it.
                    debug!("Moving Undefined {} to {}", node_id, target);
                    self.pipewire().send_message(message)?;
                }
                Some(route) => {
                    match route {
                        Some(NodeTarget::Node(ulid)) => {
                            if ulid != target {
                                // Set to the wrong target, fix it.
                                debug!("Fixing {} from {} to {}", node_id, ulid, target);
                                self.pipewire().send_message(message)?;
                            } else {
                                debug!("Node Arrived and correctly configured: {}", node_id);
                            }
                        }
                        Some(NodeTarget::UnmanagedNode(node_id)) => {
                            // Pointing to an unmanaged node, send it to the correct node
                            debug!("Sending from Unmanaged {} to {}", node_id, target);
                            self.pipewire().send_message(message)?;
                        }
                        None => {
                            // If we get here, this app has been configured to the Default device
                            debug!("Sending 'Default' {} to {}", node_id, target);
                            self.pipewire().send_message(message)?;
                        }
                    }
                }
            }
        } else {
            warn!("No Target found for {}", node_id);
        }
        Ok(())
    }

    fn application_target_changed(&mut self, id: u32, target: Target) -> Result<()> {
        // Firstly, grab the node this relates to.
        if let Some(node) = self.application_nodes.get_mut(&id) {
            // Get the Original value for this node
            let original = node.media_target;

            // Update the Media target with then new value
            node.media_target = target;

            // Find whether this should be going somewhere
            if let Some(desired) = self.get_application_assignment(id) {
                match target {
                    None => {
                        // This shouldn't theoretically occur, a target change should only be triggered
                        // when there is Some(X) attached to the Target. a media_target of None means
                        // that Pipewire has, at no point, offered up a routing location, so by
                        // extension, this shouldn't trigger.
                        warn!("None on Target Update: {} - {:?}", id, target);
                    }
                    Some(route) => {
                        match route {
                            Some(NodeTarget::Node(target)) => {
                                if target != desired {
                                    debug!("Target is not the desired output, adding to ignore..");
                                    // This node has been manually routed somewhere else, ignore it.
                                    if !self.application_target_ignore.contains(&id) {
                                        debug!("Ignoring Application Node {}", id);
                                        self.application_target_ignore.push(id);
                                    }
                                } else {
                                    // This is pointing to our desired location, manage it.
                                    debug!("Node moved to Managed Location, monitoring: {}", id);
                                    self.application_target_ignore.retain(|node| *node != id);
                                }
                            }
                            Some(NodeTarget::UnmanagedNode(id)) => {
                                if !self.application_target_ignore.contains(&id) {
                                    debug!("Ignoring Application Node {}", id);
                                    self.application_target_ignore.push(id);
                                }
                            }
                            None => {
                                if original.is_none() {
                                    debug!("Setting Initial Target for {}", id);
                                    // This is the first Pipewire message for this node.
                                    let message = SetApplicationTarget(id, desired);
                                    self.pipewire().send_message(message)?;
                                } else {
                                    // We've had a previous target, so ignore this change.
                                    if !self.application_target_ignore.contains(&id) {
                                        debug!("Ignoring Application Node {} - To Default", id);
                                        self.application_target_ignore.push(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            warn!("Received Route for Unknown Node");
        }

        // If we fall through here, this isn't a managed node, so just go on normally.
        Ok(())
    }

    fn application_volume_changed(&mut self, id: u32, volume: u8) -> Result<()> {
        if let Some(dev) = self.application_nodes.get_mut(&id) {
            dev.volume = volume;
        }
        Ok(())
    }

    fn application_title_changed(&mut self, id: u32, title: String) -> Result<()> {
        if let Some(dev) = self.application_nodes.get_mut(&id) {
            dev.title = Some(title);
        }
        Ok(())
    }
    fn application_mute_changed(&mut self, id: u32, muted: bool) -> Result<()> {
        if let Some(dev) = self.application_nodes.get_mut(&id) {
            dev.muted = muted;
        }
        Ok(())
    }

    fn application_removed(&mut self, id: u32) -> Result<()> {
        let _ = self.application_nodes.remove(&id);
        Ok(())
    }
}

trait ApplicationManagementLocal {
    fn get_application_assignment(&mut self, id: u32) -> Option<Ulid>;
    fn get_application_type_from_node(&self, id: Ulid) -> Option<DeviceType>;
    fn find_matching_nodes(&self, def: &AppDefinition) -> Vec<u32>;
}

impl ApplicationManagementLocal for PipewireManager {
    fn get_application_assignment(&mut self, id: u32) -> Option<Ulid> {
        if let Some(node) = self.application_nodes.get(&id) {
            let node_type = get_application_type(node.node_class);

            let mapping = &self.profile.application_mapping[node_type];
            if let Some(app) = mapping.get(&node.process_name) {
                if let Some(id) = app.get(&node.name) {
                    return Some(*id);
                } else {
                    debug!("App {} - {} has no entry", node.process_name, node.name);
                }
            } else {
                debug!("Process {} has No entry", node.process_name);
            }
        } else {
            warn!("Node Not Present Application Node List: {}", id);
        }
        None
    }

    fn get_application_type_from_node(&self, id: Ulid) -> Option<DeviceType> {
        match self.get_node_type(id) {
            None => None,
            Some(node_type) => match node_type {
                NodeType::PhysicalSource => Some(DeviceType::Source),
                NodeType::PhysicalTarget => Some(DeviceType::Target),
                NodeType::VirtualSource => Some(DeviceType::Source),
                NodeType::VirtualTarget => Some(DeviceType::Target),
            },
        }
    }

    fn find_matching_nodes(&self, def: &AppDefinition) -> Vec<u32> {
        let mut list = vec![];
        for (node_id, node) in &self.application_nodes {
            let node_type = get_application_type(node.node_class);

            if node.name == def.name
                && node.process_name == def.process
                && node_type == def.device_type
            {
                list.push(*node_id);
            }
        }
        list
    }
}

pub fn get_application_type(class: MediaClass) -> DeviceType {
    match class {
        MediaClass::Sink => DeviceType::Target,
        _ => DeviceType::Source,
    }
}
