use crate::commands::{APICommand, DaemonRequest, DaemonStatus};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
#[allow(unused)]
pub trait Client {
    async fn send(&mut self, request: DaemonRequest) -> Result<()>;
    async fn poll_status(&mut self) -> Result<()>;
    async fn command(&mut self, command: APICommand) -> Result<()>;
    fn status(&self) -> &DaemonStatus;
}
