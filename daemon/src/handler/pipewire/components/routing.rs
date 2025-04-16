use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use log::{debug, warn};
use pipeweaver_shared::{Mix, NodeType};
use ulid::Ulid;

pub(crate) trait RoutingManagement {
    async fn routing_load(&mut self) -> Result<()>;
    async fn routing_load_source(&mut self, source: &Ulid) -> Result<()>;
    async fn routing_load_target(&mut self, target: &Ulid) -> Result<()>;

    async fn routing_set_route(&mut self, source: Ulid, target: Ulid, enabled: bool) -> Result<()>;
    async fn routing_route_exists(&self, source: Ulid, target: Ulid) -> Result<bool>;

    async fn routing_get_target_mix(&self, id: &Ulid) -> Result<Mix>;
    async fn routing_set_target_mix(&mut self, target: Ulid, mix: Mix) -> Result<()>;
}

impl RoutingManagement for PipewireManager {
    async fn routing_load(&mut self) -> Result<()> {
        // This should be called after all the nodes are set up, we need to check our routing table
        // and establish links between the sources and targets
        debug!("Loading Routing..");

        let routing = &self.profile.routes.clone();
        for source in routing.keys() {
            self.routing_load_source(source).await?;
        }

        Ok(())
    }

    async fn routing_load_source(&mut self, source: &Ulid) -> Result<()> {
        if let Some(targets) = self.profile.routes.get(source) {
            for target in targets {
                let target_node = self.get_target_filter_node(*target)?;
                if !self.is_source_muted_to_some(*source, *target).await? {
                    if let Some(map) = self.source_map.get(source).copied() {
                        // Grab the Mix to Route From
                        let mix = self.routing_get_target_mix(target).await?;
                        self.link_create_filter_to_filter(map[mix], target_node).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn routing_load_target(&mut self, target: &Ulid) -> Result<()> {
        // This one's a little different, it's for a newly appearing target that may need routing
        for (source, targets) in &self.profile.routes {
            if targets.contains(target) && !self.is_source_muted_to_some(*source, *target).await? {
                let target_node = self.get_target_filter_node(*target)?;
                if let Some(map) = self.source_map.get(source) {
                    let mix = self.routing_get_target_mix(target).await?;
                    self.link_create_filter_to_filter(map[mix], target_node).await?;
                }
            }
        }
        Ok(())
    }


    async fn routing_set_route(&mut self, source: Ulid, target: Ulid, enabled: bool) -> Result<()> {
        // This is actually more complicated that it sounds, first lets find some stuff out..
        let source_type = self.get_node_type(source).ok_or(anyhow!("Source Not Found"))?;
        let target_type = self.get_node_type(target).ok_or(anyhow!("Target Not Found"))?;

        // Make sure the user is being sane
        if !matches!(source_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }
        if !matches!(target_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        // This should already be here, but it's not, so create it
        let target_id = self.get_target_filter_node(target)?;
        self.profile.routes.entry(source).or_insert_with(|| {
            warn!("[Routing] Table Missing for Source {}, Creating", source);
            Default::default()
        });

        // This unwrap is safe, so just grab the Set and check what we're doing
        let route = self.profile.routes.get_mut(&source).unwrap();
        if enabled == route.contains(&target) {
            bail!("Requested route change already set");
        }
        if enabled { route.insert(target); } else { route.remove(&target); }

        // Next, we need to get the A/B IDs for the Source
        if let Some(map) = self.source_map.get(&source).copied() {
            // Set up the Pipewire Links
            if enabled {
                // Only create the route if it's not currently muted
                if !self.is_source_muted_to_some(source, target).await? {
                    let mix = self.routing_get_target_mix(&target).await?;
                    self.link_create_filter_to_filter(map[mix], target_id).await?;
                }
            } else {
                let mix = self.routing_get_target_mix(&target).await?;
                self.link_remove_filter_to_filter(map[mix], target_id).await?;
            }
        } else {
            bail!("Unable to obtain volume map for Source");
        }


        Ok(())
    }

    async fn routing_route_exists(&self, source: Ulid, target: Ulid) -> Result<bool> {
        let source_type = self.get_node_type(source).ok_or(anyhow!("Source Not Found"))?;
        let target_type = self.get_node_type(target).ok_or(anyhow!("Target Not Found"))?;

        // Make sure the user is being sane
        if !matches!(source_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }
        if !matches!(target_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        if !self.profile.routes.contains_key(&source) {
            return Ok(false);
        }

        Ok(self.profile.routes.get(&source).unwrap().contains(&target))
    }

    async fn routing_get_target_mix(&self, id: &Ulid) -> Result<Mix> {
        let error = anyhow!("Cannot Locate Node");
        let node_type = self.get_node_type(*id).ok_or(error)?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        let err = anyhow!("Failed to Locate Target");
        let mix = if node_type == NodeType::PhysicalTarget {
            self.get_physical_target(*id).ok_or(err)?.mix
        } else {
            self.get_virtual_target(*id).ok_or(err)?.mix
        };
        Ok(mix)
    }

    async fn routing_set_target_mix(&mut self, target: Ulid, mix: Mix) -> Result<()> {
        let current = self.routing_get_target_mix(&target).await?;

        // Ok, first thing's first, lets see if this is actually changed
        if current == mix {
            bail!("Nothing to Do, Mixes Match");
        }

        let error = anyhow!("Cannot Locate Node");
        let node_type = self.get_node_type(target).ok_or(error)?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Target is a Source Node");
        }

        let target_node = self.get_target_filter_node(target)?;

        // Next, grab all the routes to this target
        for (source, targets) in &self.profile.routes {
            if targets.contains(&target) {
                // This source to this Target exists, check whether this route is muted
                if !self.is_source_muted_to_some(*source, target).await? {
                    // We need to detach the link from this source, and attach it to a new one
                    if let Some(map) = self.source_map.get(source).copied() {
                        // Switch the Link between the mixes
                        self.link_remove_filter_to_filter(map[current], target_node).await?;
                        self.link_create_filter_to_filter(map[mix], target_node).await?;
                    }
                }
            }
        }

        // Update the Profile
        if node_type == NodeType::PhysicalTarget {
            self.get_physical_target_mut(target).ok_or(anyhow!("Unknown Node"))?.mix = mix;
        } else {
            self.get_virtual_target_mut(target).ok_or(anyhow!("Unknown Node"))?.mix = mix;
        }
        Ok(())
    }
}
