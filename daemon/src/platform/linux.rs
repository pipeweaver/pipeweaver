use crate::stop::Stop;
use anyhow::Result;
use log::debug;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::signal::unix::{signal, SignalKind};

pub async fn spawn_platform_runtime(mut stop: Stop) -> Result<()> {
    // This one's a little odd, because Windows doesn't directly support SIGTERM, we're going
    // to monitor for it here, and trigger a shutdown if one is received.
    let mut stream = signal(SignalKind::terminate())?;

    select! {
        Ok(()) = ctrl_c() => {
                debug!("CTRL_C");
                stop.trigger();
        }
        Some(_) = stream.recv() => {
            // Trigger a Shutdown
            debug!("TERM Signal Received, Triggering STOP");
            stop.trigger();
        },
        () = stop.recv() => {
            stop.trigger();
        }
    }
    debug!("Platform Runtime Ended");
    Ok(())
}
