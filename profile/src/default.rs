use crate::{
    DeviceDescription, Devices, Mix, MuteState, MuteStates,
    PhysicalSourceDevice, PhysicalTargetDevice, Profile, SourceDevices, TargetDevices,
    VirtualSourceDevice, VirtualTargetDevice, Volumes,
};
use enum_map::enum_map;
use pipeweaver_shared::{Colour, DeviceType, OrderGroup};
use std::collections::{HashMap, HashSet};
use ulid::Ulid;

impl Profile {
    pub fn base_settings() -> Self {
        let mic_id = Ulid::new();
        let system_id = Ulid::new();
        let browser_id = Ulid::new();
        let headphones_id = Ulid::new();
        let chat_mic_id = Ulid::new();

        Self {
            devices: Devices {
                sources: SourceDevices {
                    physical_devices: vec![PhysicalSourceDevice {
                        description: DeviceDescription {
                            id: mic_id,
                            name: "Microphone".to_string(),
                            colour: Colour {
                                red: 47,
                                green: 24,
                                blue: 71,
                            },
                        },
                        mute_states: MuteStates {
                            mute_state: HashSet::new(),
                            mute_targets: Default::default(),
                        },
                        volumes: Volumes {
                            volume: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                            volumes_linked: Some(1.),
                        },
                        attached_devices: vec![],
                    }],
                    virtual_devices: vec![
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: system_id,
                                name: "System".to_string(),
                                colour: Colour {
                                    red: 153,
                                    green: 98,
                                    blue: 30,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: Default::default(),
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 100,
                                    Mix::B => 100,
                                },
                                volumes_linked: Some(1.),
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: browser_id,
                                name: "Browser".to_string(),
                                colour: Colour {
                                    red: 211,
                                    green: 139,
                                    blue: 93,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: Default::default(),
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 100,
                                    Mix::B => 100,
                                },
                                volumes_linked: Some(1.),
                            },
                        },
                    ],
                    device_order: enum_map! {
                        OrderGroup::Default => vec![
                            system_id,
                            browser_id,
                        ],
                        OrderGroup::Hidden => vec![],
                        OrderGroup::Pinned => vec![mic_id],
                    },
                },
                targets: TargetDevices {
                    physical_devices: vec![PhysicalTargetDevice {
                        description: DeviceDescription {
                            id: headphones_id,
                            name: "Headphones".to_string(),
                            colour: Default::default(),
                        },
                        mute_state: MuteState::Unmuted,
                        volume: 100,
                        mix: Mix::A,
                        attached_devices: vec![],
                    }],
                    virtual_devices: vec![VirtualTargetDevice {
                        description: DeviceDescription {
                            id: chat_mic_id,
                            name: "Chat Mic".to_string(),
                            colour: Colour {
                                red: 11,
                                green: 37,
                                blue: 69,
                            },
                        },
                        mute_state: MuteState::Unmuted,
                        volume: 100,
                        mix: Mix::A,

                        attached_devices: Default::default(),
                    }],

                    device_order: enum_map! {
                    OrderGroup::Default => vec![
                            headphones_id,
                            chat_mic_id,
                        ],
                        OrderGroup::Hidden => vec![],
                        OrderGroup::Pinned => vec![mic_id],
                    },
                },
            },
            routes: vec![
                (mic_id, [chat_mic_id].into_iter().collect()),
                (system_id, [headphones_id].into_iter().collect()),
                (browser_id, [headphones_id].into_iter().collect()),
            ]
            .into_iter()
            .collect(),

            application_mapping: enum_map! {
                DeviceType::Source => {
                    HashMap::from([
                        ("firefox".into(), HashMap::from([("Firefox".into(), browser_id)])),
                        ("chromium".into(), HashMap::from([("Chromium".into(), browser_id)])),
                        ("chrome".into(), HashMap::from([("Google Chrome".into(), browser_id)])),
                    ])
                },
                DeviceType::Target => {
                    Default::default()
                }
            },
        }
    }
}
