mod default;

use enum_map::{Enum, EnumMap};
use pipecast_shared::{Colour, Mix, MuteState, MuteTarget};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

/// Main Profile Node
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// A list of devices currently configured in this profile
    pub devices: Devices,
    pub routes: HashMap<Ulid, Vec<Ulid>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Devices {
    /// Source devices (Devices that bring audio into the Mixer)
    pub sources: SourceDevices,

    /// Target devices (Devices that audio is routed into)
    pub targets: TargetDevices,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SourceDevices {
    /// Sink Devices physically attached to Pipewire
    pub physical_devices: Vec<PhysicalSourceDevice>,

    /// Virtual Source devices
    pub virtual_devices: Vec<VirtualSourceDevice>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TargetDevices {
    /// Source Devices attached to Pipewire
    pub physical_devices: Vec<PhysicalTargetDevice>,

    /// Virtual Sink Devices
    pub virtual_devices: Vec<VirtualTargetDevice>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDescription {
    pub id: Ulid,
    pub name: String,

    pub colour: Colour,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,
    pub volumes: Volumes,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MuteStates {
    pub mute_state: MuteState,
    pub mute_targets: EnumMap<MuteTarget, Vec<Ulid>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDeviceDescriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,
    pub volumes: Volumes,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTargetDevice {
    pub description: DeviceDescription,

    pub volume: u8,
    pub mix: Mix,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalTargetDevice {
    pub description: DeviceDescription,

    pub volume: u8,
    pub mix: Mix,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Volumes {
    pub volume: EnumMap<Mix, u8>,
    pub volumes_linked: Option<f32>,
}

