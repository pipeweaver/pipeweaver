use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use log::warn;
use pipecast_shared::{Mix, NodeType};
use ulid::Ulid;

pub(crate) trait RoutingManagement {
    async fn load_routing(&mut self) -> Result<()>;

    async fn routing_set_route(&mut self, source: Ulid, target: Ulid, enabled: bool) -> Result<()>;
    async fn routing_route_exists(&self, source: Ulid, target: Ulid) -> Result<bool>;
}

impl RoutingManagement for PipewireManager {
    async fn load_routing(&mut self) -> Result<()> {
        // This should be called after all the nodes are set up, we need to check our routing table
        // and establish links between the sources and targets

        let routing = &self.profile.routes;
        for (source, targets) in routing {
            for target in targets {
                let target = self.get_target_node(*target)?;
                if !self.is_source_muted_to_some(*source, target).await? {
                    if let Some(map) = self.source_map.get(source).copied() {
                        self.link_create_filter_to_filter(map[Mix::A], target).await?;
                        self.link_create_filter_to_filter(map[Mix::B], target).await?;
                    }
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
        let target_id = self.get_target_node(target)?;
        if self.profile.routes.get(&source).is_none() {
            warn!("[Routing] Table Missing for Source {}, Creating", source);
            self.profile.routes.insert(source, Default::default());
        }

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
                    self.link_create_filter_to_filter(map[Mix::A], target_id).await?;
                    self.link_create_filter_to_filter(map[Mix::B], target_id).await?;
                }
            } else {
                self.link_remove_filter_to_filter(map[Mix::A], target_id).await?;
                self.link_remove_filter_to_filter(map[Mix::B], target_id).await?;
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
}

trait RoutingManagementLocal {}