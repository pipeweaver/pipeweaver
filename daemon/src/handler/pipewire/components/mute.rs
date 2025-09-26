use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use log::{debug, info, warn};
use pipeweaver_pipewire::PipewireMessage;
use pipeweaver_profile::MuteStates;
use pipeweaver_shared::{Mix, MuteState, MuteTarget, NodeType};
use std::collections::HashSet;
use strum::IntoEnumIterator;
use ulid::Ulid;

pub(crate) trait MuteManager {
    async fn add_target_mute_node(&mut self, id: Ulid, state: MuteTarget, target: Ulid) -> Result<()>;
    async fn del_target_mute_node(&mut self, id: Ulid, state: MuteTarget, target: Ulid) -> Result<()>;
    async fn clear_target_mute_nodes(&mut self, id: Ulid, state: MuteTarget) -> Result<()>;

    async fn set_source_mute_state(&mut self, id: Ulid, target: MuteTarget, state: MuteState) -> Result<()>;
    async fn set_target_mute_state(&mut self, id: Ulid, state: MuteState) -> Result<()>;

    async fn is_source_muted_to_some(&self, source: Ulid, target: Ulid) -> Result<bool>;
    async fn is_source_muted_to_all(&self, source: Ulid) -> Result<bool>;
    async fn get_target_mute_state(&self, target: Ulid) -> Result<MuteState>;
}

impl MuteManager for PipewireManager {
    async fn add_target_mute_node(&mut self, id: Ulid, state: MuteTarget, target: Ulid) -> Result<()> {
        let node_type = self.get_node_type(target).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        // First get the total target nodes available in the current configuration
        let target_node_count = self.get_target_node_count();

        // Check whether this target is already present in this mute state
        let mute_state = self.get_source_mute_states_mut(id)?;
        if mute_state.mute_targets[state].contains(&target) {
            bail!("Target Already in Mute Target");
        }

        // If this MuteTarget is already muted, we should 'fix' the change
        if mute_state.mute_state.contains(&state) {
            // TODO: We should just 'Update' the current mute state, but for now, unmute it
            warn!("Un-muting {:?} to ensure consistency", state);
            self.set_source_mute_state(id, state, MuteState::Unmuted).await?;
        }

        // Re-fetch the state to avoid borrow issues with calling `set_source_mute_state`
        let mute_state = self.get_source_mute_states_mut(id)?;

        // If all target nodes are added as mute targets, set it to empty
        if mute_state.mute_targets[state].len() + 1 >= target_node_count {
            warn!("All Targets Selected, Reverting back to 'Mute to All'");
            mute_state.mute_targets[state].clear();
        } else {
            // Add the target to the list
            mute_state.mute_targets[state].insert(target);
        }
        Ok(())
    }

    async fn del_target_mute_node(&mut self, id: Ulid, state: MuteTarget, target: Ulid) -> Result<()> {
        let node_type = self.get_node_type(target).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        // Check whether this target is already present in this mute state
        let mute_state = self.get_source_mute_states_mut(id)?;
        if !mute_state.mute_targets[state].contains(&target) {
            bail!("Target Not Present in Mute Target");
        }

        // If this MuteTarget is already muted, we should 'fix' the change
        if mute_state.mute_state.contains(&state) {
            // TODO: We should just 'Update' the current mute state, but for now, unmute it
            warn!("Un-muting {:?} to ensure consistency", state);
            self.set_source_mute_state(id, state, MuteState::Unmuted).await?;
        }

        // Re-fetch the state to avoid borrow issues with calling `set_source_mute_state`
        let mute_state = self.get_source_mute_states_mut(id)?;
        mute_state.mute_targets[state].remove(&target);

        Ok(())
    }

    async fn clear_target_mute_nodes(&mut self, id: Ulid, state: MuteTarget) -> Result<()> {
        let mute_state = self.get_source_mute_states_mut(id)?;

        // If this MuteTarget is already muted, we should 'fix' the change
        if mute_state.mute_state.contains(&state) {
            // TODO: We should just 'Update' the current mute state, but for now, unmute it
            warn!("Un-muting {:?} to ensure consistency", state);
            self.set_source_mute_state(id, state, MuteState::Unmuted).await?;
        }

        // Re-fetch the state to avoid borrow issues with calling `set_source_mute_state`
        let mute_state = self.get_source_mute_states_mut(id)?;
        mute_state.mute_targets[state].clear();

        Ok(())
    }

    async fn set_source_mute_state(&mut self, id: Ulid, target: MuteTarget, state: MuteState) -> Result<()> {
        // Get the Mute States for this Source
        let mute_state = self.get_source_mute_states_mut(id)?;

        // Check whether a change has actually occurred here
        if (state == MuteState::Muted) == mute_state.mute_state.contains(&target) {
            return Ok(());
        }

        // Get the current mute targets based on the current state
        let has_mute_state = !mute_state.mute_state.is_empty();
        let mute_targets = Self::get_mute_targets(mute_state);

        // Update the mute state
        match state {
            MuteState::Unmuted => mute_state.mute_state.retain(|&e| e != target),
            MuteState::Muted => { mute_state.mute_state.insert(target); }
        }

        // Let's do this again for the new values
        let has_new_mute_state = !mute_state.mute_state.is_empty();
        let new_mute_targets = Self::get_mute_targets(mute_state);

        // Handle transitions
        if !has_mute_state && has_new_mute_state {
            debug!("Transition: Unmuted → Muted");

            if new_mute_targets.is_empty() {
                self.mute_remove_volume(id).await?;
            } else {
                self.mute_remove_routes(id, &new_mute_targets).await?;
            }
        } else if has_mute_state && !has_new_mute_state {
            debug!("Transition: Muted → Unmuted");
            if mute_targets.is_empty() {
                self.mute_restore_volume(id).await?;
            } else {
                self.mute_restore_routes(id, &mute_targets).await?;
            }
        } else if has_mute_state && has_new_mute_state {
            debug!("Transition: Muted → Muted with Different Targets");

            if mute_targets.is_empty() && new_mute_targets.is_empty() {
                debug!("Already Muted to All, no changes required.");
            } else if mute_targets.is_empty() && !new_mute_targets.is_empty() {
                debug!("Transition: Muted (All) → Muted (Some)");
                self.mute_remove_routes(id, &new_mute_targets).await?;
                self.mute_restore_volume(id).await?;
            } else if !mute_targets.is_empty() && new_mute_targets.is_empty() {
                debug!("Transition: Muted (Some) → Muted (All)");
                self.mute_remove_volume(id).await?;
                self.mute_restore_routes(id, &mute_targets).await?;
            } else {
                debug!("Transition: Muted (Some) → Muted (Different Some)");
                let restore_routes = mute_targets.difference(&new_mute_targets).copied().collect();
                let remove_routes = new_mute_targets.difference(&mute_targets).copied().collect();

                self.mute_restore_routes(id, &restore_routes).await?;
                self.mute_remove_routes(id, &remove_routes).await?;
            }
        } else {
            warn!("Unexpected: Unmuted → Unmuted (No change needed)");
        }

        Ok(())
    }

    async fn set_target_mute_state(&mut self, id: Ulid, state: MuteState) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        // Attempt to Grab the 'Unmuted' Volume for this Target
        let err = anyhow!("Unable to Locate Target");
        let profile_volume = if node_type == NodeType::PhysicalTarget {
            self.get_physical_target(id).ok_or(err)?.volume
        } else {
            self.get_virtual_target(id).ok_or(err)?.volume
        };

        // Get the Current Mute state in a mutable form (we can safely unwrap here, missing nodes
        // would have failed above)
        let current_state = if node_type == NodeType::PhysicalTarget {
            let device = self.get_physical_target_mut(id).unwrap();
            &mut device.mute_state
        } else {
            let device = self.get_virtual_target_mut(id).unwrap();
            &mut device.mute_state
        };

        if current_state == &state {
            info!("No Change Made, new state is same as current state");
            return Ok(());
        }

        // Update the profile mute state
        *current_state = state;

        if node_type == NodeType::PhysicalTarget {
            // Attempt to apply the 'Muted' / 'Unmuted' volume to the filter
            match state {
                MuteState::Unmuted => self.filter_volume_set(id, profile_volume).await?,
                MuteState::Muted => self.filter_volume_set(id, 0).await?,
            }
        } else {
            // Apply mute state to Pipewire
            let message = PipewireMessage::SetNodeMute(id, match state {
                MuteState::Unmuted => false,
                MuteState::Muted => true,
            });
            let _ = self.pipewire().send_message(message);
        }

        Ok(())
    }

    async fn is_source_muted_to_some(&self, source: Ulid, target: Ulid) -> Result<bool> {
        let states = self.get_source_mute_states(source)?;
        for state in MuteTarget::iter() {
            if states.mute_state.contains(&state) {
                let targets = &states.mute_targets[state];
                if !targets.is_empty() && targets.contains(&target) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn is_source_muted_to_all(&self, source: Ulid) -> Result<bool> {
        let states = self.get_source_mute_states(source)?;
        for state in MuteTarget::iter() {
            if states.mute_state.contains(&state) && states.mute_targets[state].is_empty() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn get_target_mute_state(&self, target: Ulid) -> Result<MuteState> {
        let node_type = self.get_node_type(target).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Source is a Target Node");
        }

        let err = anyhow!("Unable to Find Target");
        let state = if node_type == NodeType::PhysicalTarget {
            &self.get_physical_target(target).ok_or(err)?.mute_state
        } else {
            &self.get_virtual_target(target).ok_or(err)?.mute_state
        };

        Ok(*state)
    }
}

trait MuteManagerLocal {
    /// A Simple Method which will iterate all available mute states and create a HashSet containing
    /// a full list of targets
    fn get_mute_targets(list: &MuteStates) -> HashSet<Ulid>;

    fn get_source_mute_states(&self, source: Ulid) -> Result<&MuteStates>;
    fn get_source_mute_states_mut(&mut self, source: Ulid) -> Result<&mut MuteStates>;

    async fn mute_remove_volume(&mut self, source: Ulid) -> Result<()>;
    async fn mute_remove_routes(&mut self, source: Ulid, targets: &HashSet<Ulid>) -> Result<()>;
    async fn mute_remove_route(&mut self, source: Ulid, target: Ulid) -> Result<()>;

    async fn mute_restore_volume(&mut self, source: Ulid) -> Result<()>;
    async fn mute_restore_routes(&mut self, source: Ulid, targets: &HashSet<Ulid>) -> Result<()>;
    async fn mute_restore_route(&mut self, source: Ulid, target: Ulid) -> Result<()>;
}

impl MuteManagerLocal for PipewireManager {
    fn get_mute_targets(state: &MuteStates) -> HashSet<Ulid> {
        // Check whether any target is empty, and assume a MuteToAll..
        if state.mute_state.iter().any(|&target| state.mute_targets[target].is_empty()) {
            return HashSet::new();
        }

        // Pull out the specific unique targets from all active Mute States
        state.mute_state.iter().flat_map(|&t| state.mute_targets[t].iter().copied()).collect()
    }

    fn get_source_mute_states(&self, source: Ulid) -> Result<&MuteStates> {
        let node_type = self.get_node_type(source).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }

        let err = anyhow!("Unable to Find Source");
        let states = if node_type == NodeType::PhysicalSource {
            &self.get_physical_source(source).ok_or(err)?.mute_states
        } else {
            &self.get_virtual_source(source).ok_or(err)?.mute_states
        };

        Ok(states)
    }

    fn get_source_mute_states_mut(&mut self, source: Ulid) -> Result<&mut MuteStates> {
        let node_type = self.get_node_type(source).ok_or(anyhow!("Unknown Node"))?;
        if !matches!(node_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }

        let err = anyhow!("Unable to Find Source");
        let states = if node_type == NodeType::PhysicalSource {
            &mut self.get_physical_source_mut(source).ok_or(err)?.mute_states
        } else {
            &mut self.get_virtual_source_mut(source).ok_or(err)?.mute_states
        };

        Ok(states)
    }

    async fn mute_remove_volume(&mut self, source: Ulid) -> Result<()> {
        let mix_err = anyhow!("Unable to Find Source Mixes");
        let map = self.source_map.get(&source).copied().ok_or(mix_err)?;

        debug!("Action: Set Volume to 0 for Channel");
        self.filter_volume_set(map[Mix::A], 0).await?;
        self.filter_volume_set(map[Mix::B], 0).await?;

        Ok(())
    }

    async fn mute_remove_routes(&mut self, source: Ulid, targets: &HashSet<Ulid>) -> Result<()> {
        for target in targets {
            debug!("Action: Remove Route to {}", target);
            if let Err(e) = self.mute_remove_route(source, *target).await {
                warn!("Cannot Remove Route: {}", e);
            }
        }
        Ok(())
    }

    async fn mute_remove_route(&mut self, source: Ulid, target: Ulid) -> Result<()> {
        let mix_err = anyhow!("Unable to Find Source Mixes");
        let map = self.source_map.get(&source).copied().ok_or(mix_err)?;

        if !self.routing_route_exists(source, target).await? {
            // We don't have a route here anyway, so nothing to remove.
            bail!("Route doesn't Exist");
        }

        let node_type = self.get_node_type(target).ok_or(anyhow!("Cannot Find Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        let target_mix = self.routing_get_target_mix(&target).await?;
        if node_type == NodeType::PhysicalTarget {
            self.link_remove_filter_to_filter(map[target_mix], target).await?;
        } else {
            self.link_remove_filter_to_node(map[target_mix], target).await?;
        }

        Ok(())
    }

    async fn mute_restore_volume(&mut self, source: Ulid) -> Result<()> {
        let mix_err = anyhow!("Unable to Find Source Mixes");
        let map = self.source_map.get(&source).copied().ok_or(mix_err)?;

        let profile_volume_a = self.get_node_volume(source, Mix::A)?;
        let profile_volume_b = self.get_node_volume(source, Mix::B)?;

        debug!("Action: Restore Volume for Channel");
        self.filter_volume_set(map[Mix::A], profile_volume_a).await?;
        self.filter_volume_set(map[Mix::B], profile_volume_b).await?;

        Ok(())
    }

    async fn mute_restore_routes(&mut self, source: Ulid, targets: &HashSet<Ulid>) -> Result<()> {
        for target in targets {
            debug!("Action: Restore Route to {}", target);
            if let Err(e) = self.mute_restore_route(source, *target).await {
                warn!("Cannot Restore Route: {}", e);
            }
        }
        Ok(())
    }

    async fn mute_restore_route(&mut self, source: Ulid, target: Ulid) -> Result<()> {
        let mix_err = anyhow!("Unable to Find Source Mixes");
        let map = self.source_map.get(&source).copied().ok_or(mix_err)?;

        if !self.routing_route_exists(source, target).await? {
            // We don't have a route here anyway, so nothing to remove.
            bail!("Route doesn't Exist");
        }

        let node_type = self.get_node_type(target).ok_or(anyhow!("Cannot Find Node"))?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        let mix = self.routing_get_target_mix(&target).await?;
        if node_type == NodeType::PhysicalTarget {
            self.link_create_filter_to_filter(map[mix], target).await?;
        } else {
            self.link_create_filter_to_node(map[mix], target).await?;
        }
        Ok(())
    }
}
