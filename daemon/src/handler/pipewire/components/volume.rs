use crate::handler::pipewire::components::filters::FilterManagement;
use crate::handler::pipewire::components::links::LinkManagement;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::profile::ProfileManagement;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{Result, anyhow, bail};
use log::debug;
use pipeweaver_pipewire::{FilterValue, PipewireMessage};
use pipeweaver_profile::Volumes;
use pipeweaver_shared::{Mix, MuteState, NodeType};
use ulid::Ulid;

pub(crate) trait VolumeManager {
    async fn volumes_load(&self) -> Result<()>;
    async fn load_initial_volume(&self, id: Ulid) -> Result<()>;

    async fn sync_pipewire_volume(&mut self, id: Ulid);
    async fn sync_pipewire_mute(&mut self, id: Ulid);

    async fn sync_all_pipewire_volumes(&mut self);
    async fn sync_all_pipewire_mutes(&mut self);

    async fn sync_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()>;
    async fn sync_node_mute(&mut self, id: Ulid, muted: bool) -> Result<()>;

    async fn set_source_volume(&mut self, id: Ulid, mix: Mix, volume: u8, api: bool) -> Result<()>;
    async fn set_source_volume_linked(&mut self, id: Ulid, linked: bool) -> Result<()>;

    async fn set_target_volume(&mut self, id: Ulid, volume: u8, from_api: bool) -> Result<()>;

    async fn set_metering(&mut self, enabled: bool) -> Result<()>;
    fn get_node_volume(&self, id: Ulid, mix: Mix) -> Result<u8>;
}

impl VolumeManager for PipewireManager {
    //
    async fn volumes_load(&self) -> Result<()> {
        // Need to go through the various node types, and call a volume set
        for device in &self.profile.devices.sources.physical_devices {
            self.load_initial_volume(device.description.id).await?;
        }
        for device in &self.profile.devices.sources.virtual_devices {
            self.load_initial_volume(device.description.id).await?;
        }
        for device in &self.profile.devices.targets.virtual_devices {
            self.load_initial_volume(device.description.id).await?;
        }
        for device in &self.profile.devices.targets.physical_devices {
            self.load_initial_volume(device.description.id).await?;
        }
        Ok(())
    }

    async fn load_initial_volume(&self, id: Ulid) -> Result<()> {
        let error = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(error)?;

        let error = anyhow!("Unable to Locate Target Node");
        match node_type {
            NodeType::PhysicalSource | NodeType::VirtualSource => {
                debug!("Loading Volume for {}", id);
                self.volume_source_load_with_mute(id).await?
            }
            NodeType::PhysicalTarget => {
                let device = self.get_physical_target(id).ok_or(error)?;
                debug!("Setting Initial Volume for {} to {}", id, device.volume);
                self.volume_target_load_with_mute(id, device.volume).await?
            }
            NodeType::VirtualTarget => {
                let device = self.get_virtual_target(id).ok_or(error)?;
                debug!("Setting Initial Volume for {} to {}", id, device.volume);
                self.volume_target_load_with_mute(id, device.volume).await?
            }
        }
        Ok(())
    }

    async fn sync_pipewire_volume(&mut self, id: Ulid) {
        if let Ok(volume) = self.get_node_volume(id, Mix::A) {
            let message = PipewireMessage::SetNodeVolume(id, volume);
            let _ = self.pipewire().send_message(message);
        }
    }

    async fn sync_pipewire_mute(&mut self, id: Ulid) {
        if let Ok(muted) = self.get_target_mute_state(id).await {
            let message = PipewireMessage::SetNodeMute(id, muted == MuteState::Muted);
            let _ = self.pipewire().send_message(message);
        }
    }

    async fn sync_all_pipewire_volumes(&mut self) {
        let source_ids: Vec<Ulid> = self.source_map.keys().copied().collect();

        for id in source_ids {
            self.sync_pipewire_volume(id).await;
        }

        let device_ids: Vec<Ulid> = self
            .profile
            .devices
            .targets
            .virtual_devices
            .iter()
            .map(|dev| dev.description.id)
            .collect();

        for dev in device_ids {
            self.sync_pipewire_volume(dev).await;
        }
    }

    async fn sync_all_pipewire_mutes(&mut self) {
        let device_ids: Vec<Ulid> = self
            .profile
            .devices
            .targets
            .virtual_devices
            .iter()
            .map(|dev| dev.description.id)
            .collect();

        for dev in device_ids {
            self.sync_pipewire_mute(dev).await;
        }
    }

    async fn sync_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        let volume = volume.clamp(0, 100);

        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        match node_type {
            NodeType::PhysicalSource | NodeType::VirtualSource => {
                debug!("Syncing Volume: {} - {}", id, volume);
                self.set_source_volume(id, Mix::A, volume, false).await?;
            }
            NodeType::PhysicalTarget | NodeType::VirtualTarget => {
                debug!("Syncing Volume: {} - {}", id, volume);
                self.set_target_volume(id, volume, false).await?;
            }
        }

        Ok(())
    }

    async fn sync_node_mute(&mut self, id: Ulid, muted: bool) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        if !matches!(node_type, NodeType::VirtualTarget) {
            // We don't need to sync here
            return Ok(());
        }

        let err = anyhow!("Node not Found");
        let dev = self.get_virtual_target_mut(id).ok_or(err)?;

        dev.mute_state = match muted {
            true => MuteState::Muted,
            false => MuteState::Unmuted,
        };

        Ok(())
    }

    async fn set_source_volume(&mut self, id: Ulid, mix: Mix, volume: u8, api: bool) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }

        // Now, pull out the correct part of the profile..
        let volumes = self.get_volumes(id)?;

        let volume_a = volumes.volume[Mix::A];

        // Set the New volume for this mix
        volumes.volume[mix] = volume;

        // Do a check to see if we're linked, and if so, prep to also update that value
        let other_mix = if mix == Mix::A { Mix::B } else { Mix::A };
        let update_other = if let Some(ratio) = volumes.volumes_linked {
            let new_volume = if mix == Mix::A {
                (volume as f32 * ratio) as u8
            } else {
                (volume as f32 / ratio) as u8
            }
            .clamp(0, 100);
            Some(new_volume)
        } else {
            None
        };

        self.volume_set_source(id, mix, volume).await?;

        // If this is coming from the API for Mix A, update the pipewire node volume
        if mix == Mix::A && api {
            let message = PipewireMessage::SetNodeVolume(id, volume);
            let _ = self.pipewire().send_message(message);
        }

        // If we're linked, update the other Mix as well
        if let Some(volume) = update_other {
            if mix == Mix::B && api {
                // Only update Volume A if it's below 100%
                if volume_a < 100 {
                    let message = PipewireMessage::SetNodeVolume(id, volume);
                    let _ = self.pipewire().send_message(message);
                }
            }

            // Set the secondary volume in the profile
            self.get_volumes(id)?.volume[other_mix] = volume;
            self.volume_set_source(id, other_mix, volume).await?;
        }

        Ok(())
    }

    async fn set_source_volume_linked(&mut self, id: Ulid, linked: bool) -> Result<()> {
        // Now, pull out the correct part of the profile...
        let volumes = self.get_volumes(id)?;

        if linked == volumes.volumes_linked.is_some() {
            bail!("Requested State matches current state");
        }

        if !linked {
            // Unlink the volumes
            volumes.volumes_linked = None;
            return Ok(());
        }

        // Pull out the A and B volumes, if either is 0, force to 1 to prevent divide by zero
        let volume_a = if volumes.volume[Mix::A] == 0 {
            1
        } else {
            volumes.volume[Mix::A]
        };
        let volume_b = if volumes.volume[Mix::B] == 0 {
            1
        } else {
            volumes.volume[Mix::B]
        };

        let ratio = volume_b as f32 / volume_a as f32;
        debug!("Setting Ratio to {}", ratio);
        volumes.volumes_linked = Some(ratio);

        Ok(())
    }

    async fn set_target_volume(&mut self, id: Ulid, volume: u8, api: bool) -> Result<()> {
        if !(0..=100).contains(&volume) {
            bail!("Volume Must be between 0 and 100");
        }
        let node_type = self.get_node_type(id).ok_or(anyhow!("Unknown Node"))?;
        if node_type == NodeType::VirtualTarget {
            // We should always change this, regardless of mute state
            if api {
                let message = PipewireMessage::SetNodeVolume(id, volume);
                self.pipewire().send_message(message)?;
            }
        } else if self.get_target_mute_state(id).await? == MuteState::Unmuted {
            self.filter_volume_set(id, volume).await?;
        }

        // We can safely unwrap here, errors will be thrown by get_target_mute_state if it's wrong
        if node_type == NodeType::PhysicalTarget {
            self.get_physical_target_mut(id).unwrap().volume = volume;
        } else {
            self.get_virtual_target_mut(id).unwrap().volume = volume;
        }

        Ok(())
    }

    async fn set_metering(&mut self, enabled: bool) -> Result<()> {
        if enabled == self.meter_enabled {
            // Nothing to do, changing to existing state.
            return Ok(());
        }

        for (&node, &meter) in &self.meter_map {
            let message = PipewireMessage::SetFilterValue(meter, 0, FilterValue::Bool(enabled));
            self.pipewire().send_message(message)?;

            let node_type = match self.get_node_type(node) {
                Some(node_type) => node_type,
                None => {
                    debug!("Failed to get Node Type for {}", node);
                    bail!("Unable to obtain node type");
                }
            };
            match node_type {
                NodeType::PhysicalSource | NodeType::PhysicalTarget => {
                    if enabled {
                        self.link_create_filter_to_filter(node, meter).await?;
                    } else {
                        self.link_remove_filter_to_filter(node, meter).await?;
                    }
                }
                NodeType::VirtualSource => {
                    if enabled {
                        self.link_create_node_to_filter(node, meter).await?;
                    } else {
                        self.link_remove_node_to_filter(node, meter).await?;
                    }
                }
                NodeType::VirtualTarget => {
                    // Virtual Targets need to be attached / detached against the volume
                    if enabled {
                        self.link_create_node_to_filter(node, meter).await?;
                    } else {
                        self.link_remove_node_to_filter(node, meter).await?;
                    }
                }
            }
        }
        self.meter_enabled = enabled;
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
            NodeType::PhysicalTarget => Ok(self.get_physical_target(id).ok_or(err)?.volume),
            NodeType::VirtualTarget => Ok(self.get_virtual_target(id).ok_or(err)?.volume),
        }
    }
}

trait VolumeManagerLocal {
    async fn volume_set_source(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()>;
    fn get_volumes(&mut self, id: Ulid) -> Result<&mut Volumes>;

    async fn volume_source_load_with_mute(&self, id: Ulid) -> Result<()>;
    async fn volume_target_load_with_mute(&self, id: Ulid, volume: u8) -> Result<()>;
}

impl VolumeManagerLocal for PipewireManager {
    async fn volume_set_source(&mut self, id: Ulid, mix: Mix, volume: u8) -> Result<()> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        if !matches!(
            node_type,
            NodeType::PhysicalSource | NodeType::VirtualSource
        ) {
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

    fn get_volumes(&mut self, id: Ulid) -> Result<&mut Volumes> {
        let node_type = self.get_node_type(id).ok_or(anyhow!("Node Not Found"))?;
        if !matches!(
            node_type,
            NodeType::PhysicalSource | NodeType::VirtualSource
        ) {
            bail!("Provided Source is a Target Node");
        }

        // Now, pull out the correct part of the profile..
        if node_type == NodeType::PhysicalSource {
            Ok(&mut self
                .get_physical_source_mut(id)
                .ok_or(anyhow!("Node not Found"))?
                .volumes)
        } else {
            Ok(&mut self
                .get_virtual_source_mut(id)
                .ok_or(anyhow!("Node not Found"))?
                .volumes)
        }
    }

    async fn volume_source_load_with_mute(&self, id: Ulid) -> Result<()> {
        let err = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(
            node_type,
            NodeType::PhysicalSource | NodeType::VirtualSource
        ) {
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

        debug!("Setting Volumes - A: {} B: {}", a, b);

        self.filter_volume_set(mixes[Mix::A], a).await?;
        self.filter_volume_set(mixes[Mix::B], b).await?;

        Ok(())
    }

    async fn volume_target_load_with_mute(&self, id: Ulid, volume: u8) -> Result<()> {
        let err = anyhow!("Unable to Locate Node");
        let node_type = self.get_node_type(id).ok_or(err)?;
        if !matches!(
            node_type,
            NodeType::PhysicalTarget | NodeType::VirtualTarget
        ) {
            bail!("Provided Source is a Source Node");
        }

        if node_type == NodeType::PhysicalTarget {
            // Physical Targets use a Volume filter
            if self.get_target_mute_state(id).await? == MuteState::Muted {
                self.filter_volume_set(id, 0).await?;
            } else {
                self.filter_volume_set(id, volume).await?;
            }
        } else {
            // Virtual Targets use pipewire
            let message = PipewireMessage::SetNodeVolume(id, volume);
            self.pipewire().send_message(message)?;
        }

        Ok(())
    }
}
