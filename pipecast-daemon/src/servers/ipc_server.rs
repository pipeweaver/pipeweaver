use crate::handler::packet::{handle_packet, Messenger};
use anyhow::{bail, Result};
use interprocess::local_socket::tokio::prelude::{LocalSocketListener, LocalSocketStream};
use interprocess::local_socket::traits::tokio::{Listener, Stream};
use interprocess::local_socket::{
    GenericFilePath, ListenerOptions, ToFsName,
};
use log::{debug, info, warn};
use pipecast_ipc::clients::ipc::ipc_socket::Socket;
use pipecast_ipc::commands::{DaemonRequest, DaemonResponse};
use std::fs;
use std::path::Path;

use crate::Stop;
static SOCKET_PATH: &str = "/tmp/pipecast.socket";

async fn ipc_tidy() -> Result<()> {
    if !Path::new(SOCKET_PATH).exists() {
        return Ok(());
    }
    let socket = SOCKET_PATH.to_fs_name::<GenericFilePath>()?;
    let connection = LocalSocketStream::connect(socket).await;

    if connection.is_err() {
        match cfg!(windows) {
            true => {
                debug!("Named Pipe not running, continuing..");
            }
            false => {
                debug!("Connection Failed. Socket File is stale, removing..");
                fs::remove_file(SOCKET_PATH)?;
            }
        }
        return Ok(());
    }

    debug!("Connected to socket, seeing if there's a Daemon on the other side..");
    let connection = connection?;

    let mut socket: Socket<DaemonResponse, DaemonRequest> = Socket::new(connection);
    if let Err(e) = socket.send(DaemonRequest::Ping).await {
        match cfg!(windows) {
            true => {
                debug!("Our named pipe is broken, something is horribly wrong..");
                bail!("Named Pipe Error: {}", e);
            }
            false => {
                debug!("Unable to send messages, removing socket..");
                fs::remove_file(SOCKET_PATH)?;
            }
        }
        return Ok(());
    }

    // If we get here, there's an active PipeCast Daemon running!
    bail!("The PipeCast Daemon is already running.");
}

pub async fn bind_socket() -> Result<LocalSocketListener> {
    ipc_tidy().await?;

    let name = SOCKET_PATH.to_fs_name::<GenericFilePath>()?;
    let opts = ListenerOptions::new().name(name.clone());
    let listener = opts.create_tokio()?;

    info!("Bound IPC Socket @ {:?}", name);
    Ok(listener)
}

pub async fn spawn_ipc_server(
    listener: LocalSocketListener,
    usb_tx: Messenger,
    mut shutdown_signal: Stop,
) {
    debug!("Running IPC Server..");
    loop {
        tokio::select! {
            Ok(connection) = listener.accept() => {
                let socket = Socket::new(connection);
                let usb_tx = usb_tx.clone();
                tokio::spawn(async move {
                    handle_connection(socket, usb_tx).await;
                });
            }
            () = shutdown_signal.recv() => {
                if !cfg!(windows) {
                    let _ = fs::remove_file(SOCKET_PATH);
                }
                return;
            }
        }
    }
}

async fn handle_connection(
    mut socket: Socket<DaemonRequest, DaemonResponse>,
    usb_tx: Messenger,
) {
    while let Some(msg) = socket.read().await {
        match msg {
            Ok(msg) => match handle_packet(msg, usb_tx.clone()).await {
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
    debug!("Disconnected {:?}", socket.address());
}
