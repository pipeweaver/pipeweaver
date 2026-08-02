#[cfg(not(feature = "sync"))]
pub mod tokio;

#[cfg(not(feature = "sync"))]
pub use tokio::ipc_socket::Socket;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "sync")]
pub use sync::ipc_socket::Socket;

pub mod ipc_client;

// Common IPC traits
use crate::commands::{DaemonRequest, DaemonResponse};
use anyhow::Result;
use std::path::Path;

#[cfg(not(feature = "sync"))]
use interprocess::local_socket::tokio::prelude::LocalSocketStream;
#[cfg(not(feature = "sync"))]
use interprocess::local_socket::traits::tokio::Stream as _;

#[cfg(feature = "sync")]
use interprocess::local_socket::prelude::LocalSocketStream;
#[cfg(feature = "sync")]
use interprocess::local_socket::traits::Stream as _;

use interprocess::local_socket::{GenericFilePath, ToFsName};

#[derive(Debug)]
#[allow(unused)]
pub struct IPCClient {
    socket: Socket<DaemonResponse, DaemonRequest>,
}

impl IPCClient {
    pub fn new(socket: Socket<DaemonResponse, DaemonRequest>) -> Self {
        Self { socket }
    }

    #[maybe_async::maybe_async]
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let name = path.as_ref().to_fs_name::<GenericFilePath>()?;
        let stream = LocalSocketStream::connect(name).await?;
        Ok(Self::new(Socket::new(stream)))
    }
}
