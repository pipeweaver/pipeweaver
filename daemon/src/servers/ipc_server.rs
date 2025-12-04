use crate::handler::packet::{handle_packet, Messenger};
use crate::servers::http_server::PatchEvent;
use crate::{Stop, APP_NAME, APP_NAME_ID};
use anyhow::{Error, Result};
use directories::BaseDirs;
use interprocess::local_socket::tokio::prelude::{LocalSocketListener, LocalSocketStream};
use interprocess::local_socket::traits::tokio::{Listener, Stream};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};
use log::{debug, info, warn};
use pipeweaver_ipc::clients::ipc::ipc_socket::Socket;
use pipeweaver_ipc::commands::{DaemonCommand, DaemonRequest, DaemonResponse};
use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::select;
use tokio::sync::broadcast::Sender;

#[derive(Debug)]
pub enum ErrorState {
    AlreadyRunning,
}

impl std::fmt::Display for ErrorState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorState::AlreadyRunning => write!(f, "Pipeweaver Already Running"),
        }
    }
}

impl std::error::Error for ErrorState {}

pub fn get_socket_path() -> Result<PathBuf> {
    let path = BaseDirs::new()
        .and_then(|base| base.runtime_dir().map(|p| p.to_path_buf()))
        .map(Ok::<PathBuf, Error>)
        .unwrap_or_else(|| {
            let tmp_dir = env::temp_dir().join(APP_NAME);
            if !tmp_dir.exists() {
                fs::create_dir_all(&tmp_dir)?;
            }
            Ok(tmp_dir)
        })?;

    let socket_path = path.join(format!("{}.socket", APP_NAME_ID));
    Ok(socket_path)
}

async fn ipc_tidy() -> Result<()> {
    let socket_path = get_socket_path()?;
    debug!("Using IPC Path: {:?}", socket_path);

    if !Path::new(&socket_path).exists() {
        return Ok(());
    }
    let socket = socket_path.clone().to_fs_name::<GenericFilePath>()?;
    let connection = LocalSocketStream::connect(socket).await;

    if connection.is_err() {
        debug!("Connection Failed. Socket File is stale, removing..");
        fs::remove_file(socket_path)?;
        return Ok(());
    }

    debug!("Connected to socket, seeing if there's a Daemon on the other side..");
    let connection = connection?;

    let mut socket: Socket<DaemonResponse, DaemonRequest> = Socket::new(connection);
    if let Err(e) = socket.send(DaemonRequest::Ping).await {
        debug!("Unable to send messages: {}, removing socket..", e);
        fs::remove_file(socket_path)?;
        return Ok(());
    }

    debug!("Daemon is active, asking it to open the interface..");
    let message = DaemonRequest::Daemon(DaemonCommand::OpenInterface);
    socket.send(message).await?;
    socket.read().await;

    // If we get here, there's an active Daemon running!
    Err(ErrorState::AlreadyRunning.into())
}

pub async fn bind_socket() -> Result<LocalSocketListener> {
    let socket_path = get_socket_path()?;
    ipc_tidy().await?;

    let name = socket_path.to_fs_name::<GenericFilePath>()?;
    let opts = ListenerOptions::new().name(name.clone());
    let listener = opts.create_tokio()?;

    info!("Bound IPC Socket @ {:?}", name);
    Ok(listener)
}

pub async fn spawn_ipc_server(
    listener: LocalSocketListener,
    usb_tx: Messenger,
    broadcast_tx: Sender<PatchEvent>,
    mut shutdown_signal: Stop,
) {
    let socket_path = format!("/tmp/{}.socket", APP_NAME);
    debug!("Running IPC Server..");
    loop {
        select! {
            Ok(connection) = listener.accept() => {
                let socket = Socket::new(connection);
                let usb_tx = usb_tx.clone();
                let broadcast_tx = broadcast_tx.clone();
                tokio::spawn(async move {
                    handle_connection(socket, broadcast_tx, usb_tx).await;
                });
            }
            () = shutdown_signal.recv() => {
                info!("[IPC] Stopping");
                let _ = fs::remove_file(socket_path);
                info!("[IPC] Stopped");
                return;
            }
        }
    }
}

async fn handle_connection(
    mut socket: Socket<DaemonRequest, DaemonResponse>,
    broadcast_tx: Sender<PatchEvent>,
    usb_tx: Messenger,
) {
    let mut subscriber = broadcast_tx.subscribe();

    loop {
        select! {
            Ok(event) = subscriber.recv() => {
                let patch = DaemonResponse::Patch(event.data);
                if let Err(e) = socket.send(patch).await {
                    warn!("Couldn't send PatchEvent to {:?}: {}", socket.address(), e);
                    return;
                }
            }
            Some(msg) = socket.read() => {
                match msg {
                    Ok(msg) => match handle_packet(msg, &usb_tx).await {
                        Ok(response) => {
                            if let Err(e) = socket.send(response).await {
                                warn!("Couldn't reply to {:?}: {}", socket.address(), e);
                                return;
                            }
                        }
                        Err(e) => {
                            if let Err(e) = socket.send(DaemonResponse::Err(e.to_string())).await {
                                warn!("Couldn't reply to {:?}: {}", socket.address(), e);
                                return;
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Invalid message from {:?}: {}", socket.address(), e);
                        if let Err(e) = socket.send(DaemonResponse::Err(e.to_string())).await {
                            warn!("Could not reply to {:?}: {}", socket.address(), e);
                            return;
                        }
                    }
                }
            }
            else => {
                break;
            }
        }
    }

    debug!("Disconnected {:?}", socket.address());
}
