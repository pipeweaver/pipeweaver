use crate::stop::Stop;
use anyhow::Result;

pub(crate) mod linux;

pub async fn spawn_runtime(stop: Stop) -> Result<()> {
    linux::spawn_platform_runtime(stop).await
}