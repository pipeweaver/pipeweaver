mod default;

use enum_map::{enum_map, EnumMap};
use pipeweaver_shared::{Application, Colour, Mix, MuteState, MuteTarget, OrderGroup};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use ulid::Ulid;

/// Main Profile Node
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// A list of devices currently configured in this profile
    pub devices: Devices,
    pub routes: HashMap<Ulid, HashSet<Ulid>>,

    #[serde(default)]
    pub application_mapping: HashMap<Application, Ulid>,
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

    /// Device Orders
    pub device_order: EnumMap<OrderGroup, Vec<Ulid>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TargetDevices {
    /// Source Devices attached to Pipewire
    pub physical_devices: Vec<PhysicalTargetDevice>,

    /// Virtual Sink Devices
    pub virtual_devices: Vec<VirtualTargetDevice>,

    /// Device Orders
    pub device_order: EnumMap<OrderGroup, Vec<Ulid>>,
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
    pub mute_state: HashSet<MuteTarget>,
    pub mute_targets: EnumMap<MuteTarget, HashSet<Ulid>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDeviceDescriptor {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,
    pub volumes: Volumes,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTargetDevice {
    pub description: DeviceDescription,

    pub mute_state: MuteState,
    pub volume: u8,
    pub mix: Mix,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalTargetDevice {
    pub description: DeviceDescription,

    pub mute_state: MuteState,
    pub volume: u8,
    pub mix: Mix,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

impl Default for PhysicalTargetDevice {
    fn default() -> Self {
        Self {
            description: Default::default(),
            volume: 100,

            mute_state: Default::default(),
            mix: Default::default(),

            attached_devices: Default::default(),
        }
    }
}

impl Default for VirtualTargetDevice {
    fn default() -> Self {
        Self {
            description: Default::default(),
            volume: 100,

            mute_state: Default::default(),
            mix: Default::default(),

            attached_devices: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volumes {
    pub volume: EnumMap<Mix, u8>,
    pub volumes_linked: Option<f32>,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes {
            volume: enum_map! {
                Mix::A => 100,
                Mix::B => 100,
            },
            volumes_linked: Some(1.),
        }
    }
}

