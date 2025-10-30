use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Result;
use globset::Glob;
use log::{debug, warn};
use pipeweaver_pipewire::PipewireMessage::SetApplicationTarget;
use pipeweaver_pipewire::{ApplicationNode, PipewireMessage, RouteTarget};
use pipeweaver_shared::ApplicationMatch;
use ulid::Ulid;

type Target = Option<Option<RouteTarget>>;
pub(crate) trait ApplicationManagement {
    fn set_application_target(&mut self, title: ApplicationMatch, target: Ulid) -> Result<()>;
    fn clear_application_target(&mut self, title: ApplicationMatch) -> Result<()>;
    fn set_application_transient_target(&mut self, id: u32, target: Ulid) -> Result<()>;

    fn application_appeared(&mut self, node: ApplicationNode) -> Result<()>;
    fn application_target_changed(&mut self, id: u32, target: Target) -> Result<()>;
    fn application_volume_changed(&mut self, id: u32, volume: u8) -> Result<()>;
    fn application_title_changed(&mut self, id: u32, title: String) -> Result<()>;
    fn application_removed(&mut self, id: u32) -> Result<()>;
}

impl ApplicationManagement for PipewireManager {
    fn set_application_target(&mut self, title: ApplicationMatch, target: Ulid) -> Result<()> {
        todo!()
    }

    fn clear_application_target(&mut self, title: ApplicationMatch) -> Result<()> {
        todo!()
    }

    fn set_application_transient_target(&mut self, id: u32, target: Ulid) -> Result<()> {
        // This code sets a 'transient' target for a node, and flags it as ignored.
        let message = SetApplicationTarget(id, target);
        self.pipewire().send_message(message)?;

        // Flag all future changes to this node as 'ignored'
        if !self.application_target_ignore.contains(&id) {
            self.application_target_ignore.push(id);
        }
        Ok(())
    }

    fn application_appeared(&mut self, node: ApplicationNode) -> Result<()> {
        let node_id = node.node_id;
        let node_target = node.media_target;

        self.application_nodes.insert(node_id, node);
        if self.application_target_ignore.contains(&node_id) {
            debug!("Target node is ignored, we're done here.");
            return Ok(());
        }

        // Check whether this node is assigned in the profile
        if let Some(target) = self.check_assignment(node_id) {
            // Compare against the existing target..
            let message = SetApplicationTarget(node_id, target);

            match node_target {
                None => {
                    // We've not received a RouteTarget for this app, so redirect it.
                    debug!("Moving Undefined {} to {}", node_id, target);
                    self.pipewire().send_message(message)?;
                }
                Some(route) => {
                    match route {
                        Some(RouteTarget::Node(ulid)) => {
                            if ulid != target {
                                // Set to the wrong target, fix it.
                                debug!("Fixing {} from {} to {}", node_id, ulid, target);
                                self.pipewire().send_message(message)?;
                            }
                        }
                        Some(RouteTarget::UnmanagedNode(node_id)) => {
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
            if let Some(desired) = self.check_assignment(id) {
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
                            Some(RouteTarget::Node(target)) => {
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
                            Some(RouteTarget::UnmanagedNode(id)) => {
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

    fn application_removed(&mut self, id: u32) -> Result<()> {
        let _ = self.application_nodes.remove(&id);
        Ok(())
    }
}

trait ApplicationManagementLocal {
    fn check_assignment(&mut self, id: u32) -> Option<Ulid>;
    fn get_assignment(&mut self, name: String) -> Option<Ulid>;
}

impl ApplicationManagementLocal for PipewireManager {
    fn check_assignment(&mut self, id: u32) -> Option<Ulid> {
        if let Some(node) = self.application_nodes.get(&id) {
            // Grab the application name, and see if we match
            return self.get_assignment(node.name.clone());
        }
        warn!("Application assignment not found: {}", id);
        None
    }

    fn get_assignment(&mut self, node_name: String) -> Option<Ulid> {
        for matcher in &self.profile.application_mapping {
            match matcher {
                ApplicationMatch::Exact(name, target) => {
                    if node_name == *name {
                        // Got an exact name match, send it
                        return Some(*target);
                    }
                }
                ApplicationMatch::Glob(name, target) => {
                    if Glob::new(name)
                        .unwrap()
                        .compile_matcher()
                        .is_match(&node_name)
                    {
                        return Some(*target);
                    }
                }
            }
        }
        None
    }
}
