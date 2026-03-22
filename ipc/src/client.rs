use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
#[allow(unused)]
pub trait Client {
    async fn send(&mut self, request: DaemonRequest) -> Result<DaemonResponse>;
    async fn get_status(&mut self) -> Result<DaemonStatus>;
}
