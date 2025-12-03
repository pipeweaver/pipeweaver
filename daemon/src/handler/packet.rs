use anyhow::{Context, Result, anyhow};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::handler::messaging::DaemonMessage;
use pipeweaver_ipc::commands::{DaemonRequest, DaemonResponse};

pub type Messenger = Sender<DaemonMessage>;
type Response = Result<DaemonResponse>;

/// This is pretty similar to the GoXLR Utility, as very little really needs to change here.
pub async fn handle_packet(request: DaemonRequest, sender: Messenger) -> Response {
    // Ok, we just match the request, and send it off where it needs to go..
    match request {
        DaemonRequest::Ping => Ok(DaemonResponse::Ok),
        DaemonRequest::GetStatus => {
            let (tx, rx) = oneshot::channel();

            sender
                .send(DaemonMessage::GetStatus(tx))
                .await
                .map_err(|e| anyhow!(e.to_string()))
                .context("Failed to send message to device manager")?;

            let result = rx.await.context("Error from device manager")?;
            Ok(DaemonResponse::Status(result))
        }
        DaemonRequest::Daemon(daemon_command) => {
            let (tx, rx) = oneshot::channel();
            sender
                .send(DaemonMessage::RunDaemon(daemon_command, tx))
                .await
                .map_err(|e| anyhow!(e.to_string()))
                .context("Failed to send message to device manager")?;

            rx.await.context("Error from device manager")?;
            Ok(DaemonResponse::Ok)
        }
        DaemonRequest::Pipewire(command) => {
            let (tx, rx) = oneshot::channel();
            sender
                .send(DaemonMessage::RunPipewire(command, tx))
                .await
                .map_err(|e| anyhow!(e.to_string()))
                .context("Failed to send message to device manager")?;

            let result = rx.await.context("Error from Device Manager")?;
            Ok(DaemonResponse::Pipewire(result))
        }
    }
}
