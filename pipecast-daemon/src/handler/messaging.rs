use tokio::sync::oneshot;

use pipecast_ipc::commands::{DaemonCommand, DaemonResponse, DaemonStatus, PipewireCommand, PipewireCommandResponse};

pub enum DaemonMessage {
    GetStatus(oneshot::Sender<DaemonStatus>),
    RunDaemon(DaemonCommand, oneshot::Sender<DaemonResponse>),
    RunPipewire(PipewireCommand, oneshot::Sender<PipewireCommandResponse>),
}
