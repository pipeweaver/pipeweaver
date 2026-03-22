use crate::commands::{APICommand, DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::{Result, anyhow};
use async_trait::async_trait;

#[async_trait]
#[allow(unused)]
pub trait Client {
    async fn send(&mut self, request: DaemonRequest) -> Result<DaemonResponse>;
    async fn get_status(&mut self) -> Result<DaemonStatus> {
        let status = self.send(DaemonRequest::GetStatus).await?;
        match status {
            DaemonResponse::Status(status) => Ok(status),
            DaemonResponse::Err(error) => Err(anyhow!("{}", error)),
            _ => Err(anyhow!("Expected Status response, got {:?}", status)),
        }
    }
}
