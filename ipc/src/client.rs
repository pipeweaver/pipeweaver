use crate::commands::{DaemonRequest, DaemonResponse, DaemonStatus};
use anyhow::Result;

#[maybe_async::maybe_async(?Send)]
pub trait Client {
    async fn send(&mut self, request: &DaemonRequest) -> Result<DaemonResponse>;
    async fn get_status(&mut self) -> Result<DaemonStatus>;
}
