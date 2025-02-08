use crate::commands::{DaemonRequest, DaemonStatus, PipewireCommand};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Client {
    async fn send(&mut self, request: DaemonRequest) -> Result<()>;
    async fn poll_status(&mut self) -> Result<()>;
    async fn command(&mut self, command: PipewireCommand) -> Result<()>;
    fn status(&self) -> &DaemonStatus;
}
