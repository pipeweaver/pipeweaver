use crate::{
    DeviceDescription, Devices, Mix, MuteState, MuteStates, PhysicalDeviceDescriptor,
    PhysicalSourceDevice, PhysicalTargetDevice, Profile, SourceDevices, TargetDevices,
    VirtualSourceDevice, VirtualTargetDevice, Volumes,
};
use enum_map::enum_map;
use pipeweaver_shared::{Colour, MuteTarget, OrderGroup};
use std::collections::HashSet;
use ulid::Ulid;

impl Profile {
    pub fn base_settings() -> Self {
        let mic_id = Ulid::new();
        let pc_line_in_id = Ulid::new();
        let system_id = Ulid::new();
        let browser_id = Ulid::new();
        let game_id = Ulid::new();
        let music_id = Ulid::new();
        let chat_id = Ulid::new();
        let headphones_id = Ulid::new();
        let stream_mix_id = Ulid::new();
        let vod_mix_id = Ulid::new();
        let chat_mic_id = Ulid::new();

        Self {
            devices: Devices {
                sources: SourceDevices {
                    physical_devices: vec![
                        PhysicalSourceDevice {
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
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: Some(1.),
                            },
                            attached_devices: vec![
                                PhysicalDeviceDescriptor {
                                    name: None,
                                    description: Some(String::from("BEACN Mic Microphone")),
                                },
                                PhysicalDeviceDescriptor {
                                    name: None,
                                    description: Some(String::from("Elgato XLR Dock Mono")),
                                },
                            ],
                        },
                        PhysicalSourceDevice {
                            description: DeviceDescription {
                                id: pc_line_in_id,
                                name: "PC Line In".to_string(),
                                colour: Colour {
                                    red: 98,
                                    green: 17,
                                    blue: 99,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: Default::default(),
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: Some(1.),
                            },
                            attached_devices: vec![PhysicalDeviceDescriptor {
                                name: Some(String::from(
                                    "alsa_input.pci-0000_31_00.4.analog-stereo",
                                )),
                                description: None,
                            }],
                        },
                    ],
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
                                    Mix::A => 99,
                                    Mix::B => 99,
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
                                mute_targets: enum_map! {
                                    MuteTarget::TargetA => [
                                        stream_mix_id
                                    ].into_iter().collect(),
                                    MuteTarget::TargetB => [].into_iter().collect(),
                                },
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: None,
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: game_id,
                                name: "Game".to_string(),
                                colour: Colour {
                                    red: 243,
                                    green: 255,
                                    blue: 182,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: enum_map! {
                                    MuteTarget::TargetA => [
                                        stream_mix_id
                                    ].into_iter().collect(),
                                    MuteTarget::TargetB => [
                                        headphones_id,
                                    ].into_iter().collect(),
                                },
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: Some(1.),
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: music_id,
                                name: "Music".to_string(),
                                colour: Colour {
                                    red: 115,
                                    green: 158,
                                    blue: 130,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: Default::default(),
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: Some(1.),
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: chat_id,
                                name: "Chat".to_string(),
                                colour: Colour {
                                    red: 44,
                                    green: 85,
                                    blue: 48,
                                },
                            },
                            mute_states: MuteStates {
                                mute_state: HashSet::new(),
                                mute_targets: Default::default(),
                            },
                            volumes: Volumes {
                                volume: enum_map! {
                                    Mix::A => 99,
                                    Mix::B => 99,
                                },
                                volumes_linked: Some(1.),
                            },
                        },
                    ],
                    device_order: enum_map! {
                        OrderGroup::Default => vec![
                            system_id,
                            chat_id,
                            browser_id,
                            game_id,
                            music_id,
                            pc_line_in_id,
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
                        volume: 99,
                        mix: Mix::A,
                        attached_devices: vec![
                            PhysicalDeviceDescriptor {
                                name: None,
                                description: Some(String::from("BEACN Mic Headphones")),
                            },
                            PhysicalDeviceDescriptor {
                                name: None,
                                description: Some(String::from("GoXLR System")),
                            },
                            PhysicalDeviceDescriptor {
                                name: None,
                                description: Some(String::from("Elgato XLR Dock Analog Stereo")),
                            },
                        ],
                    }],
                    virtual_devices: vec![
                        VirtualTargetDevice {
                            description: DeviceDescription {
                                id: stream_mix_id,
                                name: "Stream Mix".to_string(),
                                colour: Colour {
                                    red: 19,
                                    green: 64,
                                    blue: 116,
                                },
                            },
                            mute_state: MuteState::Unmuted,
                            volume: 99,
                            mix: Mix::B,

                            attached_devices: Default::default(),
                        },
                        VirtualTargetDevice {
                            description: DeviceDescription {
                                id: vod_mix_id,
                                name: "VOD".to_string(),
                                colour: Colour {
                                    red: 19,
                                    green: 49,
                                    blue: 92,
                                },
                            },
                            mute_state: MuteState::Unmuted,
                            volume: 99,
                            mix: Mix::B,

                            attached_devices: Default::default(),
                        },
                        VirtualTargetDevice {
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
                            volume: 99,
                            mix: Mix::A,

                            attached_devices: Default::default(),
                        },
                    ],

                    device_order: enum_map! {
                    OrderGroup::Default => vec![
                            headphones_id,
                            chat_mic_id,
                            stream_mix_id,
                            vod_mix_id
                        ],
                        OrderGroup::Hidden => vec![],
                        OrderGroup::Pinned => vec![mic_id],
                    },
                },
            },
            routes: vec![
                (
                    mic_id,
                    [headphones_id, stream_mix_id, chat_mic_id]
                        .into_iter()
                        .collect(),
                ),
                (
                    pc_line_in_id,
                    [headphones_id, stream_mix_id].into_iter().collect(),
                ),
                (
                    chat_id,
                    [headphones_id, stream_mix_id].into_iter().collect(),
                ),
                (
                    music_id,
                    [headphones_id, stream_mix_id].into_iter().collect(),
                ),
                (
                    game_id,
                    [headphones_id, stream_mix_id].into_iter().collect(),
                ),
                (system_id, [headphones_id].into_iter().collect()),
            ]
                .into_iter()
                .collect(),

            application_mapping: Default::default(),
        }
    }
}
