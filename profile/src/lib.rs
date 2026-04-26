mod default;

use enum_map::{EnumMap, enum_map};
use pipeweaver_shared::{
    Colour, DeviceType, FilterValue, Mix, MuteState, MuteTarget, OrderGroup, Quantum,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use ulid::Ulid;

/// Main Profile Node
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// A list of devices currently configured in this profile
    pub devices: Devices,
    pub routes: HashMap<Ulid, HashSet<Ulid>>,

    /// The expected Quantum of the audio devices
    #[serde(default = "default_audio_quantum")]
    pub audio_quantum: Quantum,

    #[serde(default)]
    pub application_mapping: EnumMap<DeviceType, HashMap<String, HashMap<String, Ulid>>>,
}

fn default_audio_quantum() -> Quantum {
    Quantum::Quantum512
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Devices {
    /// Source devices (Devices that bring audio into the Mixer)
    pub sources: SourceDevices,

    /// Target devices (Devices that audio is routed into)
    pub targets: TargetDevices,

    /// Port Mapping for physical devices to stereo ports
    #[serde(default)]
    pub physical_device_port_maps: EnumMap<DeviceType, Vec<PortMap>>,
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

    #[serde(default)]
    pub filters: Vec<Filter>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MuteStates {
    pub mute_state: HashSet<MuteTarget>,
    pub mute_targets: EnumMap<MuteTarget, HashSet<Ulid>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct PhysicalDeviceDescriptor {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,
    pub volumes: Volumes,

    #[serde(default)]
    pub filters: Vec<Filter>,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,

    #[serde(default)]
    pub attached_port_maps: Vec<Ulid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualTargetDevice {
    pub description: DeviceDescription,

    pub mute_state: MuteState,
    pub volume: u8,
    pub mix: Mix,

    #[serde(default)]
    pub filters: Vec<Filter>,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,

    #[serde(default)]
    pub attached_port_maps: Vec<Ulid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalTargetDevice {
    pub description: DeviceDescription,

    pub mute_state: MuteState,
    pub volume: u8,
    pub mix: Mix,

    #[serde(default)]
    pub filters: Vec<Filter>,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,

    #[serde(default)]
    pub attached_port_maps: Vec<Ulid>,
}

impl Default for PhysicalTargetDevice {
    fn default() -> Self {
        Self {
            description: Default::default(),
            volume: 100,

            mute_state: Default::default(),
            mix: Default::default(),

            filters: Default::default(),

            attached_devices: Default::default(),
            attached_port_maps: Default::default(),
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

            filters: Default::default(),

            attached_devices: Default::default(),
            attached_port_maps: Default::default(),
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

/// This aids in allowing port mapping to occur for devices which aren't stereo to allow us
/// to connect them to the tree based on some user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMap {
    /// The physical device we're mapping
    pub device: PhysicalDeviceDescriptor,

    /// The configuration options for this device
    pub configuration: Vec<PortConfiguration>,
}

/// A Device Port Configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfiguration {
    /// The number of expected ports on this device to be usable
    pub num_ports: u32,

    /// A list of port assignments from the device, there can be many!
    pub assignments: Vec<PortAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAssignment {
    pub id: Ulid,
    pub name: String,

    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filter {
    LV2(LV2Filter),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LV2Filter {
    // We'll generate a ulid if one isn't provided
    #[serde(default = "generate_uid")]
    pub id: Ulid,

    pub plugin_uri: String,
    pub values: HashMap<String, FilterValue>,
}

fn generate_uid() -> Ulid {
    Ulid::new()
}
