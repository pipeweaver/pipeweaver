use json_patch::Patch;
use pipecast_profile::Profile;
use pipecast_shared::{Colour, Mix, MuteState, MuteTarget, NodeType};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonRequest {
    /// Simple ping, will get an Ok / Error response
    Ping,

    /// This fetches the full status for all devices
    GetStatus,

    Daemon(DaemonCommand),
    Pipewire(PipeCastCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketRequest {
    pub id: u64,
    pub data: DaemonRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonResponse {
    Ok,
    Err(String),
    Patch(Patch),
    Status(DaemonStatus),
    Pipewire(PipewireCommandResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketResponse {
    pub id: u64,
    pub data: DaemonResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonCommand {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipeCastCommand {
    CreateNode(NodeType, String),
    SetNodeColour(Ulid, Colour),
    RemoveNode(Ulid),

    SetSourceVolume(Ulid, Mix, u8),
    SetSourceVolumeLinked(Ulid, bool),
    SetTargetVolume(Ulid, u8),
    SetTargetMix(Ulid, Mix),

    SetRoute(Ulid, Ulid, bool),

    AddSourceMuteTarget(Ulid, MuteTarget),
    DelSourceMuteTarget(Ulid, MuteTarget),

    AddMuteTargetNode(Ulid, MuteTarget, Ulid),
    DelMuteTargetNode(Ulid, MuteTarget, Ulid),
    ClearMuteTargetNodes(Ulid, MuteTarget),

    SetTargetMuteState(Ulid, MuteState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipewireCommandResponse {
    Ok,
    Id(Ulid),
    Err(String),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub config: DaemonConfig,
    pub audio: AudioConfiguration,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AudioConfiguration {
    pub profile: Profile,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub http_settings: HttpSettings,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HttpSettings {
    pub enabled: bool,
    pub bind_address: String,
    pub cors_enabled: bool,
    pub port: u16,
}