mod default;

use enum_map::{Enum, EnumMap};
use ulid::Ulid;

/// Main Profile Node
pub struct Profile {
    /// A list of devices currently configured in this profile
    devices: Devices,

}

struct Devices {
    /// Source devices (Devices that bring audio into the Mixer)
    sources: SourceDevices,

    /// Target devices (Devices that audio is routed into)
    targets: TargetDevices,
}

struct SourceDevices {
    /// Sink Devices physically attached to Pipewire
    physical_devices: Vec<PhysicalSourceDevice>,

    /// Virtual Source devices
    virtual_devices: Vec<VirtualSourceDevice>,
}

struct TargetDevices {
    /// Source Devices attached to Pipewire
    physical_devices: Vec<PhysicalTargetDevice>,

    /// Virtual Sink Devices
    virtual_devices: Vec<VirtualTargetDevice>,
}

struct VirtualSourceDevice {
    id: Ulid,
    name: String,

    mute_state: MuteState,
    mute_targets: EnumMap<MuteTarget, Vec<Ulid>>,

    volumes: EnumMap<Mix, u8>,
}

struct PhysicalSourceDevice {
    id: Ulid,
    name: String,

    mute_state: MuteState,
    mute_targets: EnumMap<MuteTarget, Vec<Ulid>>,

    volumes: EnumMap<Mix, u8>,
    attached_devices: Vec<String>,
}

struct VirtualTargetDevice {
    id: Ulid,
    name: String,

    volume: u8,
    mix: Mix,
}

struct PhysicalTargetDevice {
    id: Ulid,
    name: String,

    volume: u8,
    mix: Mix,

    attached_devices: Vec<String>,
}

#[derive(Enum)]
enum Mix {
    A,
    B,
}

enum MuteState {
    Unmuted,
    MuteTargetA,
    MuteTargetB,
}

#[derive(Enum)]
enum MuteTarget {
    TargetA,
    TargetB,
}