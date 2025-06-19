use enum_map::EnumMap;
use json_patch::Patch;
use pipeweaver_profile::Profile;
use pipeweaver_shared::{Colour, DeviceType, Mix, MuteState, MuteTarget, NodeType, OrderGroup};
use serde::{Deserialize, Serialize};
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
    SetMetering(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APICommand {
    CreateNode(NodeType, String),
    RenameNode(Ulid, String),
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

    // Attach or Detach physical nodes
    AttachPhysicalNode(Ulid, u32),
    RemovePhysicalNode(Ulid, usize),

    // Set the position of a node in the order tree
    SetOrderGroup(Ulid, OrderGroup),
    SetOrder(Ulid, u8),
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

/// The API generally doesn't need to care about all the general minutia of how a Pipewire
/// node actually looks, so instead we just have a very simple Device object that provides
/// an ID to be passed back to the daemon in IPC calls, and the nodes name.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PhysicalDevice {
    pub node_id: u32,
    pub name: Option<String>,
    pub description: Option<String>,
}