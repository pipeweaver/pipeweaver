use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use pipecast_shared::{Mix, MuteState, NodeType};
use ulid::Ulid;

pub(crate) trait VolumeManager {
    async fn set_source_node_volume(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()>;
    async fn set_target_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()>;
    fn get_node_volume(&self, id: Ulid, mix: Mix) -> Result<u8>;
}

impl VolumeManager for PipewireManager {
    async fn set_source_node_volume(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }

        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        if !matches!(node_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }

        // First, check whether we need to apply this change to the filter
        if !self.is_source_muted_to_all(id).await? {
            // Locate the filter that matches this id + mix
            if let Some(map) = self.source_map.get(&id) {
                let filter_id = map[mix];
                self.filter_volume_set(filter_id, volume).await?;
            } else {
                bail!("Source not found in the Source Map");
            }
        }

        // Now, pull out the correct part of the profile..
        let volumes = if node_type == NodeType::PhysicalSource {
            &mut self.get_physical_source_mut(id).ok_or(anyhow!("Node not Found"))?.volumes
        } else {
            &mut self.get_virtual_source_mut(id).ok_or(anyhow!("Node not Found"))?.volumes
        };

        // Set the New volume for this mix
        volumes.volume[mix] = volume;

        Ok(())
    }

    async fn set_target_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        if self.get_target_mute_state(id).await? == MuteState::Unmuted {
            if node_type == NodeType::PhysicalTarget {
                self.filter_volume_set(id, volume).await?;
            } else {
                let err = anyhow!("Unable to Find Device in Target Map");
                let target = self.target_map.get(&id).ok_or(err)?;
                self.filter_volume_set(*target, volume).await?;
            }
        }

        // We can safely unwrap here, errors will be thrown by get_target_mute_state if it's wrong
        if node_type == NodeType::PhysicalTarget {
            self.get_physical_target_mut(id).unwrap().volume = volume;
        } else {
            self.get_virtual_target_mut(id).unwrap().volume = volume;
        }

        Ok(())
    }


    fn get_node_volume(&self, id: Ulid, mix: Mix) -> Result<u8> {
        let err = anyhow!("Node not Found: {}", id);
        let node_type = self.get_node_type(id).ok_or(err)?;

        let err = anyhow!("Unable to Locate Node");
        match node_type {
            NodeType::PhysicalSource => {
                Ok(self.get_physical_source(id).ok_or(err)?.volumes.volume[mix])
            }
            NodeType::VirtualSource => {
                Ok(self.get_virtual_source(id).ok_or(err)?.volumes.volume[mix])
            }
            NodeType::PhysicalTarget => {
                Ok(self.get_physical_target(id).ok_or(err)?.volume)
            }
            NodeType::VirtualTarget => {
                Ok(self.get_virtual_target(id).ok_or(err)?.volume)
            }
        }
    }
}
