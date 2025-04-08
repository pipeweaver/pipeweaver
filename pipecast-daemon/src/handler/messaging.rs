use tokio::sync::oneshot;

use pipecast_ipc::commands::{DaemonCommand, DaemonResponse, DaemonStatus, PipeCastCommand, PipewireCommandResponse};

pub enum DaemonMessage {
    GetStatus(oneshot::Sender<DaemonStatus>),
    RunDaemon(DaemonCommand, oneshot::Sender<DaemonResponse>),
    RunPipewire(PipeCastCommand, oneshot::Sender<PipewireCommandResponse>),
}
