use tokio::sync::oneshot;

use pipeweaver_ipc::commands::{
    APICommand, DaemonCommand, DaemonResponse, DaemonStatus, PWCommandResponse,
};

pub enum DaemonMessage {
    GetStatus(oneshot::Sender<DaemonStatus>),
    RunDaemon(DaemonCommand, oneshot::Sender<DaemonResponse>),
    RunPipewire(APICommand, oneshot::Sender<PWCommandResponse>),
}
