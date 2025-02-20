use crate::{DeviceDescription, Devices, Mix, MuteState, MuteStates, PhysicalSourceDevice, PhysicalTargetDevice, Profile, SourceDevices, TargetDevices, VirtualSourceDevice, VirtualTargetDevice};
use enum_map::enum_map;
use ulid::Ulid;

impl Default for Profile {
    fn default() -> Self {
        let mic_id = Ulid::from_string("01JKMZFMP9A8J92S631RF3AP3W").expect("Unable to Parse ULID");
        let system_id = Ulid::from_string("01JKMZFMP9QKHFTAJYC92HCXTV").expect("Unable to Parse ULID");
        let game_id = Ulid::from_string("01JKMZFMP940258X2W86A1FQMT").expect("Unable to Parse ULID");
        let music_id = Ulid::from_string("01JKMZFMP9HHCDABBKGV038EMB").expect("Unable to Parse ULID");
        let chat_id = Ulid::from_string("01JKMZFMP9Z4X6V73PQXWB786K").expect("Unable to Parse ULID");
        let headphones_id = Ulid::from_string("01JKMZFMP9EMT8MFS30M8KP2FZ").expect("Unable to Parse ULID");
        let stream_mix_id = Ulid::from_string("01JKMZFMP9XRDWX1QWBED7BB4T").expect("Unable to Parse ULID");
        let chat_mic_id = Ulid::from_string("01JKMZFMP9RNMBGFMN6A9ER279").expect("Unable to Parse ULID");

        Self {
            devices: Devices {
                sources: SourceDevices {
                    physical_devices: vec![
                        PhysicalSourceDevice {
                            description: DeviceDescription {
                                id: mic_id,
                                name: "Microphone".to_string(),
                            },
                            mute_states: MuteStates {
                                mute_state: MuteState::Unmuted,
                                mute_targets: Default::default(),
                            },
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                            attached_devices: vec![],
                        }
                    ],
                    virtual_devices: vec![
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: system_id,
                                name: "System".to_string(),
                            },
                            mute_states: MuteStates {
                                mute_state: MuteState::Unmuted,
                                mute_targets: Default::default(),
                            },
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: game_id,
                                name: "Game".to_string(),
                            },
                            mute_states: MuteStates {
                                mute_state: MuteState::Unmuted,
                                mute_targets: Default::default(),
                            },
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: music_id,
                                name: "Music".to_string(),
                            },
                            mute_states: MuteStates {
                                mute_state: MuteState::Unmuted,
                                mute_targets: Default::default(),
                            },
                            volumes: enum_map! {
                                Mix::A => 100,
                                Mix::B => 100,
                            },
                        },
                        VirtualSourceDevice {
                            description: DeviceDescription {
                                id: chat_id,
                                name: "Chat".to_string(),
                            },
                            mute_states: MuteStates {
                                mute_state: MuteState::Unmuted,
                                mute_targets: Default::default(),
                            },
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
                            description: DeviceDescription {
                                id: headphones_id,
                                name: "Headphones".to_string(),
                            },
                            volume: 100,
                            mix: Mix::A,
                            attached_devices: vec![],
                        },
                    ],
                    virtual_devices: vec![
                        VirtualTargetDevice {
                            description: DeviceDescription {
                                id: stream_mix_id,
                                name: "Stream Mix".to_string(),
                            },
                            volume: 100,
                            mix: Mix::B,
                        },
                        VirtualTargetDevice {
                            description: DeviceDescription {
                                id: chat_mic_id,
                                name: "Chat Mic".to_string(),
                            },
                            volume: 100,
                            mix: Mix::A,
                        }
                    ],
                },
            },
            routes: vec![
                (mic_id, vec![headphones_id, stream_mix_id, chat_mic_id]),
                (chat_id, vec![headphones_id, stream_mix_id]),
                (music_id, vec![headphones_id, stream_mix_id]),
                (game_id, vec![headphones_id, stream_mix_id]),
                (system_id, vec![headphones_id]),
            ].into_iter().collect(),
        }
    }
}