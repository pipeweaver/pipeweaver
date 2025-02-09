use crate::{Devices, Mix, MuteState, PhysicalSourceDevice, PhysicalTargetDevice, Profile, SourceDevices, TargetDevices, VirtualSourceDevice, VirtualTargetDevice};
use enum_map::enum_map;
use ulid::Ulid;

impl Default for Profile {
    fn default() -> Self {
        Self {
            devices: Devices {
                sources: SourceDevices {
                    physical_devices: vec![
                        PhysicalSourceDevice {
                            id: Ulid::from_string("01JKMZFMP9A8J92S631RF3AP3W").expect("Unable to Parse ULID"),
                            name: "Microphone".to_string(),
                            mute_state: MuteState::Unmuted,
                            mute_targets: Default::default(),
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                            attached_devices: vec![],
                        }
                    ],
                    virtual_devices: vec![
                        VirtualSourceDevice {
                            id: Ulid::from_string("01JKMZFMP9QKHFTAJYC92HCXTV").expect("Unable to Parse ULID"),
                            name: "System".to_string(),
                            mute_state: MuteState::Unmuted,
                            mute_targets: Default::default(),
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            id: Ulid::from_string("01JKMZFMP940258X2W86A1FQMT").expect("Unable to Parse ULID"),
                            name: "Game".to_string(),
                            mute_state: MuteState::Unmuted,
                            mute_targets: Default::default(),
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            id: Ulid::from_string("01JKMZFMP9HHCDABBKGV038EMB").expect("Unable to Parse ULID"),
                            name: "Music".to_string(),
                            mute_state: MuteState::Unmuted,
                            mute_targets: Default::default(),
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            id: Ulid::from_string("01JKMZFMP9Z4X6V73PQXWB786K").expect("Unable to Parse ULID"),
                            name: "Chat".to_string(),
                            mute_state: MuteState::Unmuted,
                            mute_targets: Default::default(),
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                    ],
                },
                targets: TargetDevices {
                    physical_devices: vec![
                        PhysicalTargetDevice {
                            id: Ulid::from_string("01JKMZFMP9EMT8MFS30M8KP2FZ").expect("Unable to Parse ULID"),
                            name: "Headphones".to_string(),
                            volume: 100,
                            mix: Mix::A,
                            attached_devices: vec![],
                        },
                    ],
                    virtual_devices: vec![
                        VirtualTargetDevice {
                            id: Ulid::from_string("01JKMZFMP9XRDWX1QWBED7BB4T").expect("Unable to Parse ULID"),
                            name: "Stream Mix".to_string(),
                            volume: 100,
                            mix: Mix::B,
                        },
                        VirtualTargetDevice {
                            id: Ulid::from_string("01JKMZFMP9RNMBGFMN6A9ER279").expect("Unable to Parse ULID"),
                            name: "Chat Mic".to_string(),
                            volume: 100,
                            mix: Mix::A,
                        }
                    ],
                },
            },
        }
    }
}