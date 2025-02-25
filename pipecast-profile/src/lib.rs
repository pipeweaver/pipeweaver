mod default;

use enum_map::{Enum, EnumMap};
use std::collections::HashMap;
use strum_macros::{Display, EnumIter};
use ulid::Ulid;

/// Main Profile Node
#[derive(Debug, Clone)]
pub struct Profile {
    /// A list of devices currently configured in this profile
    pub devices: Devices,
    pub routes: HashMap<Ulid, Vec<Ulid>>,
}

#[derive(Debug, Clone)]
pub struct Devices {
    /// Source devices (Devices that bring audio into the Mixer)
    pub sources: SourceDevices,

    /// Target devices (Devices that audio is routed into)
    pub targets: TargetDevices,
}

#[derive(Debug, Clone)]
pub struct SourceDevices {
    /// Sink Devices physically attached to Pipewire
    pub physical_devices: Vec<PhysicalSourceDevice>,

    /// Virtual Source devices
    pub virtual_devices: Vec<VirtualSourceDevice>,
}

#[derive(Debug, Clone)]
pub struct TargetDevices {
    /// Source Devices attached to Pipewire
    pub physical_devices: Vec<PhysicalTargetDevice>,

    /// Virtual Sink Devices
    pub virtual_devices: Vec<VirtualTargetDevice>,
}

#[derive(Debug, Clone)]
pub struct DeviceDescription {
    pub id: Ulid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct VirtualSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,
    pub volumes: EnumMap<Mix, u8>,
}

#[derive(Debug, Clone)]
pub struct MuteStates {
    pub mute_state: MuteState,
    pub mute_targets: EnumMap<MuteTarget, Vec<Ulid>>,
}

#[derive(Debug, Clone)]
pub struct PhysicalDeviceDescriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PhysicalSourceDevice {
    pub description: DeviceDescription,
    pub mute_states: MuteStates,

    pub volumes: EnumMap<Mix, u8>,
    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Debug, Clone)]
pub struct VirtualTargetDevice {
    pub description: DeviceDescription,

    pub volume: u8,
    pub mix: Mix,
}

#[derive(Debug, Clone)]
pub struct PhysicalTargetDevice {
    pub description: DeviceDescription,

    pub volume: u8,
    pub mix: Mix,

    pub attached_devices: Vec<PhysicalDeviceDescriptor>,
}

#[derive(Debug, Display, Copy, Clone, Enum, EnumIter)]
pub enum Mix {
    A,
    B,
}

#[derive(Debug, Copy, Clone)]
pub enum MuteState {
    Unmuted,
    MuteTargetA,
    MuteTargetB,
}

#[derive(Debug, Copy, Clone, Enum, EnumIter)]
pub enum MuteTarget {
    TargetA,
    TargetB,
}