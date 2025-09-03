use crate::handler::packet::Messenger;
use crate::stop::Stop;
use anyhow::Result;

pub(crate) mod linux;
pub(crate) mod tray;

pub async fn spawn_runtime(stop: Stop) -> Result<()> {
    linux::spawn_platform_runtime(stop).await
}
pub async fn spawn_tray(stop: Stop, sender: Messenger) -> Result<()> {
    tray::spawn_tray(stop, sender).await
}
