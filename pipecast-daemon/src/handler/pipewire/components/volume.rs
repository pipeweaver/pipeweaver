use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{anyhow, bail, Result};
use pipecast_shared::{Mix, MuteState, NodeType};
use ulid::Ulid;

pub(crate) trait VolumeManager {
    async fn volumes_load(&self) -> Result<()>;

    async fn set_source_node_volume(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()>;
    async fn set_target_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()>;
    fn get_node_volume(&self, id: Ulid, mix: Mix) -> Result<u8>;
}

impl VolumeManager for PipewireManager {
    async fn volumes_load(&self) -> Result<()> {
        // Need to go through the various node types, and call a volume set
        for device in &self.profile.devices.sources.physical_devices {
            self.volume_source_load_with_mute(device.description.id).await?;
        }
        for device in &self.profile.devices.sources.virtual_devices {
            self.volume_source_load_with_mute(device.description.id).await?;
        }
        for device in &self.profile.devices.targets.virtual_devices {
            self.volume_target_load_with_mute(device.description.id, device.volume).await?;
        }
        for device in &self.profile.devices.targets.physical_devices {
            self.volume_target_load_with_mute(device.description.id, device.volume).await?;
        }
        Ok(())
    }

    async fn set_source_node_volume(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }
        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        if !matches!(node_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }

        // Now, pull out the correct part of the profile..
        let volumes = if node_type == NodeType::PhysicalSource {
            &mut self.get_physical_source_mut(id).ok_or(anyhow!("Node not Found"))?.volumes
        } else {
            &mut self.get_virtual_source_mut(id).ok_or(anyhow!("Node not Found"))?.volumes
        };

        // Set the New volume for this mix
        volumes.volume[mix] = volume;
        let other_mix = if mix == Mix::A { Mix::B } else { Mix::A };

        let update_other = if let Some(ratio) = volumes.volumes_linked {
            let new_volume = if mix == Mix::A {
                (volume as f32 * ratio) as u8
            } else {
                (volume as f32 / ratio) as u8
            };
            volumes.volume[other_mix] = new_volume;
            Some(new_volume)
        } else {
            None
        };

        self.volume_set_source(id, mix, volume).await?;
        if let Some(volume) = update_other {
            self.volume_set_source(id, other_mix, volume).await?;
        }

        Ok(())
    }

    async fn set_target_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        if self.get_target_mute_state(id).await? == MuteState::Unmuted {
            let filter_target = self.get_target_filter_node(id)?;
            self.filter_volume_set(filter_target, volume).await?;
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

trait VolumeManagerLocal {
    async fn volume_set_source(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()>;

    async fn volume_source_load_with_mute(&self, id: Ulid) -> Result<()>;
    async fn volume_target_load_with_mute(&self, id: Ulid, volume: u8) -> Result<()>;
}

impl VolumeManagerLocal for PipewireManager {
    async fn volume_set_source(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()> {
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
        Ok(())
    }


    async fn volume_source_load_with_mute(&self, id: Ulid) -> Result<()> {
        let err = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(node_type, NodeType::PhysicalSource | NodeType::VirtualSource) {
            bail!("Provided Source is a Target Node");
        }

        let err = anyhow!("Unable to Locate Mixes for Node");
        let mixes = self.source_map.get(&id).ok_or(err)?;

        let (a, b) = if self.is_source_muted_to_all(id).await? {
            (0, 0)
        } else {
            let err = anyhow!("Unable to Find Node");
            let volumes = if node_type == NodeType::PhysicalSource {
                &self.get_physical_source(id).ok_or(err)?.volumes
            } else {
                &self.get_virtual_source(id).ok_or(err)?.volumes
            };
            (volumes.volume[Mix::A], volumes.volume[Mix::B])
        };

        self.filter_volume_set(mixes[Mix::A], a).await?;
        self.filter_volume_set(mixes[Mix::B], b).await?;

        Ok(())
    }

    async fn volume_target_load_with_mute(&self, id: Ulid, volume: u8) -> Result<()> {
        let err = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(node_type, NodeType::PhysicalTarget | NodeType::VirtualTarget) {
            bail!("Provided Source is a Source Node");
        }

        let target = self.get_target_filter_node(id)?;
        if self.get_target_mute_state(id).await? == MuteState::Muted {
            self.filter_volume_set(target, 0).await
        } else {
            self.filter_volume_set(target, volume).await
        }
    }
}
