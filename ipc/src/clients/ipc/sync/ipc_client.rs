use crate::client::Client;
use crate::clients::ipc::IPCClient;
use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::{Context, Result, anyhow};

impl Client for IPCClient {
    fn send(&mut self, request: &DaemonRequest) -> Result<DaemonResponse> {
        self.socket
            .send(request.clone())
            .context("Failed to send a command to the GoXLR daemon process")?;

        self.socket
            .read()
            .context("Failed to retrieve the command result from the GoXLR daemon process")?
            .context("Failed to parse the command result from the GoXLR daemon process")
    }

    fn get_status(&mut self) -> Result<DaemonStatus> {
        let status = self.send(&DaemonRequest::GetStatus)?;
        match status {
            DaemonResponse::Status(status) => Ok(status),
            DaemonResponse::Err(error) => Err(anyhow!("{}", error)),
            _ => Err(anyhow!("Expected Status response, got {:?}", status)),
        }
    }
}
