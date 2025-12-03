use crate::client::Client;
use crate::clients::ipc::ipc_socket::Socket;
use crate::commands::{APICommand, DaemonRequest, DaemonResponse, DaemonStatus, HttpSettings};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

#[derive(Debug)]
#[allow(unused)]
pub struct IPCClient {
    socket: Socket<DaemonResponse, DaemonRequest>,
    status: DaemonStatus,
    http_settings: HttpSettings,
}

impl IPCClient {
    pub fn new(socket: Socket<DaemonResponse, DaemonRequest>) -> Self {
        Self {
            socket,
            status: DaemonStatus::default(),
            http_settings: Default::default(),
        }
    }
}

#[async_trait]
impl Client for IPCClient {
    async fn send(&mut self, request: DaemonRequest) -> Result<()> {
        self.socket
            .send(request)
            .await
            .context("Failed to send a command to the GoXLR daemon process")?;
        let result = self
            .socket
            .read()
            .await
            .context("Failed to retrieve the command result from the GoXLR daemon process")?
            .context("Failed to parse the command result from the GoXLR daemon process")?;

        match result {
            DaemonResponse::Status(status) => {
                self.status = status.clone();
                self.http_settings = status.config.http_settings;
                Ok(())
            }
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Err(error) => Err(anyhow!("{}", error)),
            DaemonResponse::Patch(_patch) => {
                Err(anyhow!("Received Patch as response, shouldn't happen!"))
            }
            DaemonResponse::Pipewire(_response) => {
                // TODO: We need a way to pass back a response to a request properly..
                Ok(())
            }
        }
    }

    async fn poll_status(&mut self) -> Result<()> {
        self.send(DaemonRequest::GetStatus).await
    }

    async fn command(&mut self, command: APICommand) -> Result<()> {
        self.send(DaemonRequest::Pipewire(command))
            .await
    }

    fn status(&self) -> &DaemonStatus {
        &self.status
    }
}
