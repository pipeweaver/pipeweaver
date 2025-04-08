use crate::commands::{DaemonRequest, DaemonStatus, PipeCastCommand};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Client {
    async fn send(&mut self, request: DaemonRequest) -> Result<()>;
    async fn poll_status(&mut self) -> Result<()>;
    async fn command(&mut self, command: PipeCastCommand) -> Result<()>;
    fn status(&self) -> &DaemonStatus;
}
