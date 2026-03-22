use enum_map::EnumMap;
use json_patch::Patch;
use pipeweaver_profile::Profile;
use pipeweaver_shared::{
    AppDefinition, AppTarget, Colour, DeviceType, Mix, MuteState, MuteTarget, NodeType, OrderGroup,
    Quantum,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonRequest {
    /// Simple ping, will get an Ok / Error response
    Ping,

    /// This fetches the full status for all devices
    GetStatus,

    Daemon(DaemonCommand),
    Pipewire(APICommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketRequest {
    pub id: u64,
    pub data: DaemonRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum DaemonResponse {
    Ok,
    Err(String),
    Patch(Patch),
    Status(DaemonStatus),
    Pipewire(APICommandResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketResponse {
    pub id: u64,
    pub data: DaemonResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonCommand {
    SetAutoStart(bool),
    SetAudioQuantum(Quantum),
    SetMetering(bool),
    OpenInterface,
    ResetAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APICommand {
    CreateNode(NodeType, String),
    RenameNode(Ulid, String),
    RenameNodeByName(String, String),

    SetNodeColour(Ulid, Colour),
    SetNodeColourByName(String, Colour),

    RemoveNode(Ulid),
    RemoveNodeByName(String),

    SetVolume(Ulid, Option<Mix>, u8),
    SetSourceVolume(Ulid, Mix, u8),
    SetTargetVolume(Ulid, u8),
    SetVolumeByName(String, Option<Mix>, u8),

    SetSourceVolumeLinked(Ulid, bool),

    SetTargetMix(Ulid, Mix),
    SetTargetMixByName(String, Mix),

    SetRoute(Ulid, Ulid, bool),
    SetRouteBySourceName(String, Ulid, bool),
    SetRouteByTargetName(Ulid, String, bool),
    SetRouteByNames(String, String, bool),

    AddSourceMuteTarget(Ulid, MuteTarget),
    AddSourceMuteTargetByName(String, MuteTarget),
    DelSourceMuteTarget(Ulid, MuteTarget),
    DelSourceMuteTargetByName(String, MuteTarget),

    AddMuteTargetNode(Ulid, MuteTarget, Ulid),
    AddMuteTargetNodeBySourceName(String, MuteTarget, Ulid),
    AddMuteTargetNodeByTargetName(Ulid, MuteTarget, String),
    AddMuteTargetNodeByNames(String, MuteTarget, String),

    DelMuteTargetNode(Ulid, MuteTarget, Ulid),
    DelMuteTargetNodeBySourceName(String, MuteTarget, Ulid),
    DelMuteTargetNodeByTargetName(Ulid, MuteTarget, String),
    DelMuteTargetNodeByNames(String, MuteTarget, String),

    ClearMuteTargetNodes(Ulid, MuteTarget),
    ClearMuteTargetNodesByName(String, MuteTarget),

    SetTargetMuteState(Ulid, MuteState),
    SetTargetMuteStatesByName(String, MuteState),

    // Attach or Detach physical nodes
    AttachPhysicalNode(Ulid, u32),
    AttachPhysicalNodeByName(String, u32),

    RemovePhysicalNode(Ulid, usize),
    RemovePhysicalNodeByName(String, usize),

    // Used for Application Routing
    SetApplicationRoute(AppDefinition, Ulid),
    SetApplicationRouteByName(AppDefinition, String),
    ClearApplicationRoute(AppDefinition),

    SetTransientApplicationRoute(u32, Ulid),
    SetTransientApplicationRouteByName(u32, String),
    ClearTransientApplicationRoute(u32),

    SetApplicationVolume(u32, u8),
    SetApplicationMute(u32, bool),

    // Set the position of a node in the order tree
    SetOrderGroup(Ulid, OrderGroup),
    SetOrder(Ulid, u8),
    SetOrderByName(String, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APICommandResponse {
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
    pub devices: EnumMap<DeviceType, Vec<PhysicalDevice>>,
    pub defaults: EnumMap<DeviceType, Option<AppTarget>>,
    pub applications: EnumMap<DeviceType, HashMap<String, HashMap<String, Vec<Application>>>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub http_settings: HttpSettings,
    pub auto_start: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HttpSettings {
    pub enabled: bool,
    pub bind_address: String,
    pub cors_enabled: bool,
    pub port: u16,
}

/// The API generally doesn't need to care about all the general minutia of how a Pipewire
/// node actually looks, so instead we just have a very simple Device object that provides
/// an ID to be passed back to the daemon in IPC calls, and the nodes name.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PhysicalDevice {
    pub node_id: u32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_usable: bool,
}

/// This will be extended over time, for now we'll just include the node id and the name.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Application {
    pub node_id: u32,
    pub name: String,

    pub volume: u8,
    pub muted: bool,
    pub title: Option<String>,

    pub target: Option<AppTarget>,
}
