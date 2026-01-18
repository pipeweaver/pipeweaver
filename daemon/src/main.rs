mod handler;
mod platform;
mod servers;
mod settings;
mod stop;

use crate::handler::messaging::DaemonMessage;
use crate::handler::primary_worker::start_primary_worker;
use crate::platform::{spawn_runtime, spawn_tray};
use crate::servers::http_server::spawn_http_server;
use crate::servers::ipc_server::{ErrorState, bind_socket, spawn_ipc_server};
use crate::stop::Stop;
use anyhow::{Context, Result, anyhow, bail};
use directories::ProjectDirs;
use file_rotate::compression::Compression;
use file_rotate::suffix::AppendCount;
use file_rotate::{ContentLimit, FileRotate};
use log::{LevelFilter, error, info};
use pipeweaver_ipc::commands::{DaemonCommand, HttpSettings};
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, SharedLogger, TermLogger, TerminalMode, WriteLogger,
};
use std::env;
use std::fs::create_dir_all;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::{join, task};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HASH: &str = env!("GIT_HASH");

const BACKGROUND_PARAM: &str = "--background";
const LEGACY_BACKGROUND_PARAM: &str = "--startup";

// Definitions used during node / filter declarations
const APP_ID: &str = "io.github.pipeweaver";
const APP_NAME: &str = "PipeWeaver";
const APP_NAME_ID: &str = "pipeweaver";
const APP_DAEMON_NAME: &str = "pipeweaver-daemon";
const ICON: &[u8] = include_bytes!("../resources/icons/pipeweaver-large.png");

#[tokio::main]
async fn main() -> Result<()> {
    let dirs = ProjectDirs::from("io", "github", APP_NAME_ID)
        .ok_or(anyhow!("Unable to locate project directory"))?;

    // Set up Logging
    let mut log_targets: Vec<Box<dyn SharedLogger>> = vec![];
    let log_dir = dirs.data_dir().join("logs");
    create_dir_all(&log_dir).context("Could not create logs directory")?;

    // We need to ignore a couple of packages log output so create a builder.
    let mut log_config = ConfigBuilder::new();

    // The tracing package, when used, will output to INFO from zbus every second
    log_config.add_filter_ignore_str("tracing");

    // Actix is a little noisy on startup and shutdown, so quiet it a bit :)
    log_config.add_filter_ignore_str("actix_server::accept");
    log_config.add_filter_ignore_str("actix_server::worker");
    log_config.add_filter_ignore_str("actix_server::server");
    log_config.add_filter_ignore_str("actix_server::builder");
    log_config.add_filter_ignore_str("zbus");

    let log_file = log_dir.join("pipeweaver.log");
    println!("Logging to file: {log_file:?}");

    let file_rotate = FileRotate::new(
        log_file,
        AppendCount::new(5),
        ContentLimit::Bytes(1024 * 1024 * 2),
        Compression::OnRotate(1),
        #[cfg(unix)]
        None,
    );
    log_targets.push(WriteLogger::new(
        LevelFilter::Debug,
        log_config.build(),
        file_rotate,
    ));
    log_targets.push(TermLogger::new(
        LevelFilter::Debug,
        log_config.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ));

    CombinedLogger::init(log_targets)?;
    info!("Starting {} v{} - {}", APP_NAME, VERSION, HASH);

    let shutdown = Stop::new();
    let (broadcast_tx, broadcast_rx) = broadcast::channel(16);

    // Create the Global Manager Channels...
    let (manager_send, manager_recv) = mpsc::channel(32);

    // Prepare the IPC Socket
    let ipc_socket = bind_socket().await;
    if let Err(e) = ipc_socket {
        match e.downcast_ref() {
            Some(ErrorState::AlreadyRunning) => {
                info!("Pipeweaver already running, triggering interface.");
                return Ok(());
            }
            _ => {
                error!("Error Starting Daemon: {}", e);
                bail!("Error Starting Daemon: {}", e);
            }
        }
    }

    let ipc_socket = ipc_socket?;
    let communications_handle = tokio::spawn(spawn_ipc_server(
        ipc_socket,
        manager_send.clone(),
        broadcast_tx.clone(),
        shutdown.clone(),
    ));

    // Prepare the HTTP Server
    let http_settings = HttpSettings {
        enabled: true,
        bind_address: "0.0.0.0".to_string(),
        cors_enabled: false,
        port: 14565,
    };

    let (httpd_tx, httpd_rx) = tokio::sync::oneshot::channel();
    let (meter_tx, meter_rx) = broadcast::channel(32);
    drop(broadcast_rx);
    drop(meter_rx);

    tokio::spawn(spawn_http_server(
        manager_send.clone(),
        httpd_tx,
        broadcast_tx.clone(),
        meter_tx.clone(),
        http_settings,
    ));
    let http_server = httpd_rx.await?;

    let config_dir = dirs.config_dir().to_path_buf();
    let task = task::spawn(start_primary_worker(
        manager_recv,
        shutdown.clone(),
        broadcast_tx.clone(),
        meter_tx.clone(),
        config_dir,
    ));

    let args: Vec<String> = env::args().collect();
    let hide_initial = args.contains(&BACKGROUND_PARAM.to_string())
        || args.contains(&LEGACY_BACKGROUND_PARAM.to_string());

    if !hide_initial {
        let (tx, rx) = oneshot::channel();
        let message = DaemonMessage::RunDaemon(DaemonCommand::OpenInterface, tx);
        let _ = manager_send.send(message).await;
        let _ = rx.await;
    }

    let tray_icon = task::spawn(spawn_tray(shutdown.clone(), manager_send.clone()));
    let runtime = task::spawn(spawn_runtime(shutdown.clone()));

    // Wait for a Shutdown Trigger
    let _ = shutdown.clone().recv().await;

    // Join on the Threads until they all end
    let _ = join!(
        task,
        communications_handle,
        runtime,
        tray_icon,
        http_server.stop(false)
    );

    Ok(())
}
