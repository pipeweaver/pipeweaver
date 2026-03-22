use crate::client::Client;
use crate::clients::ipc::ipc_socket::Socket;
use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;

#[derive(Debug)]
#[allow(unused)]
pub struct IPCClient {
    socket: Socket<DaemonResponse, DaemonRequest>,
}

impl IPCClient {
    pub fn new(socket: Socket<DaemonResponse, DaemonRequest>) -> Self {
        Self { socket }
    }
}

#[async_trait]
impl Client for IPCClient {
    async fn send(&mut self, request: DaemonRequest) -> Result<DaemonResponse> {
        self.socket
            .send(request)
            .await
            .context("Failed to send a command to the GoXLR daemon process")?;

        self.socket
            .read()
            .await
            .context("Failed to retrieve the command result from the GoXLR daemon process")?
            .context("Failed to parse the command result from the GoXLR daemon process")
    }
}
