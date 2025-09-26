use crate::stop::Stop;
use anyhow::Result;
use log::info;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::signal::unix::{signal, SignalKind};

pub async fn spawn_platform_runtime(mut stop: Stop) -> Result<()> {
    // This one's a little odd, because Windows doesn't directly support SIGTERM, we're going
    // to monitor for it here, and trigger a shutdown if one is received.
    let mut stream = signal(SignalKind::terminate())?;

    select! {
        Ok(()) = ctrl_c() => {
                info!("[Platform] Got Ctrl+C, Stopping...");
                stop.trigger();
        }
        Some(_) = stream.recv() => {
            // Trigger a Shutdown
            info!("[Platform] Got Signal, Stopping...");
            stop.trigger();
        },
        () = stop.recv() => {
            stop.trigger();
        }
    }
    info!("[Platform] Stopped");
    Ok(())
}
