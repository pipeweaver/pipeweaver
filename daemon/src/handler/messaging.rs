use tokio::sync::oneshot;

use pipeweaver_ipc::commands::{
    APICommand, APICommandResponse, DaemonCommand, DaemonResponse, DaemonStatus,
};

pub enum DaemonMessage {
    GetStatus(oneshot::Sender<DaemonStatus>),
    RunDaemon(DaemonCommand, oneshot::Sender<DaemonResponse>),
    RunPipewire(APICommand, oneshot::Sender<APICommandResponse>),
}
