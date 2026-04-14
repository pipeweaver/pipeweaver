use anyhow::{Error, Result, anyhow, bail};
use cpp::cpp;
use directories::BaseDirs;
use log::{debug, error, info, warn};
use qmetaobject::QObjectPinned;
use qmetaobject::prelude::*;
use qmetaobject::webengine;
use std::cell::RefCell;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use std::{env, fs, thread};
use tungstenite::http::Uri;
use tungstenite::{Message, connect};

mod raise_window;
mod window_handler;
mod window_properties;

use crate::raise_window::raise_window;
use crate::window_handler::{WindowHandler, WindowMessage};
use window_properties::WindowProperties;

const APP_NAME: &str = "pipeweaver-app";

/// How long the IPC thread waits for Qt to pong back before declaring it hung.
const QT_PING_TIMEOUT: Duration = Duration::from_millis(200);

/// How long handle_active_instance waits for any reply from the existing instance.
const IPC_REPLY_TIMEOUT: Duration = Duration::from_secs(1);

cpp! {{
    #include <QGuiApplication>
    #include <QIcon>
    #include <QString>
}}

qrc!(pipeweaver_resources,
    "webengine" {
        "main.qml",
        "resources/pipeweaver.svg",
    },
);

fn main() -> Result<()> {
    if let Err(e) = real_main() {
        display_error(format!("{e}"));
        bail!(e);
    }

    Ok(())
}

fn real_main() -> Result<()> {
    let mut flags = vec![
        "--disable-software-rasterizer",
        "--disable-dev-shm-usage",
        "--disable-background-networking",
        "--disable-renderer-backgrounding",
        "--js-flags=--expose-gc,--max-old-space-size=192",
        "--num-raster-threads=2",
        "--single-process",
    ];

    // Disable the Chromium Sandbox when running from a flatpak, as flatpak handles
    // the sandboxing, and this can conflict
    if env::var("FLATPAK_SANDBOX_DIR").is_ok() {
        flags.push("--no-sandbox");
        flags.push("--disable-setuid-sandbox");
    }

    unsafe {
        //env::set_var("QT_QPA_PLATFORM", "xcb");
        env::set_var("RUST_LOG", "debug");
        env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", flags.join(" "));
    }
    env_logger::init();

    if handle_active_instance() {
        println!("Instance Already active, Exiting");
        return Ok(());
    }

    // Write our PID so a future instance can kill us if we become broken.
    write_pid_file();

    // Channel for notifications from code to the Window
    let (notify_tx, notify_rx) = mpsc::channel();

    // Ok, lets try getting the websocket running
    let (res_tx, res_rx) = mpsc::channel();
    let notify_websocket = notify_tx.clone();
    thread::spawn(move || {
        websocket_main_thread(res_tx, notify_websocket);
    });

    if let Err(e) = res_rx.recv()? {
        error!("Failed to Connect to Pipeweaver: {e}");
        bail!("Cannot Start, Pipeweaver is not running.   ");
    }

    webengine::initialize();
    pipeweaver_resources();

    // Configure QT to pick the relevant desktop file
    unsafe {
        cpp!([] {
            QGuiApplication::setDesktopFileName("pipeweaver-app");
            QGuiApplication::setWindowIcon(QIcon(QString(":/webengine/resources/pipeweaver.svg")));
        });
    }

    // Spawn the IPC thread with only the sender (thread must NOT touch QObjects)
    thread::spawn(move || {
        if let Err(e) = ipc_thread_main(notify_tx) {
            warn!("IPC thread exited with error: {e}");
        }
    });

    // Create the engine and link up the rust side
    let mut engine = QmlEngine::new();

    let window_props = Rc::new(RefCell::new(WindowProperties::new()));
    let ipc_handler = Rc::new(RefCell::new(WindowHandler::new(notify_rx)));
    unsafe {
        engine.set_object_property(
            "windowProperties".into(),
            QObjectPinned::new(window_props.as_ref()),
        );

        engine.set_object_property(
            "windowHandler".into(),
            QObjectPinned::new(ipc_handler.as_ref()),
        );
    }

    engine.load_file("qrc:/webengine/main.qml".into());
    engine.exec();

    // Clean up runtime files on a clean Qt exit.
    let _ = fs::remove_file(get_socket_file_path());
    let _ = fs::remove_file(get_pid_file_path());

    Ok(())
}

fn websocket_main_thread(res: mpsc::Sender<Result<()>>, tx: mpsc::Sender<WindowMessage>) {
    let uri = match Uri::builder()
        .authority("localhost:14565")
        .scheme("ws")
        .path_and_query("/api/websocket")
        .build()
    {
        Ok(uri) => uri,
        Err(e) => {
            let _ = res.send(Err(anyhow!(e)));
            return;
        }
    };

    info!("Attempting to connect to Pipeweaver at {uri}");
    let (mut socket, response) = match connect(uri) {
        Ok((socket, response)) => (socket, response),
        Err(e) => {
            let _ = res.send(Err(anyhow!(e)));
            return;
        }
    };

    info!("Connected, HTTP status: {}", response.status());
    let _ = res.send(Ok(()));

    loop {
        match socket.read() {
            Ok(msg) => match msg {
                Message::Ping(payload) => {
                    let _ = socket.send(Message::Pong(payload));
                }
                Message::Close(_) => {
                    println!("Server closed the connection");
                    break;
                }
                _ => {}
            },
            Err(tungstenite::Error::ConnectionClosed) => {
                error!("Disconnected: connection closed");
                break;
            }
            Err(tungstenite::Error::Protocol(e)) => {
                error!("Disconnected: protocol error: {e}");
                break;
            }
            Err(e) => {
                error!("Disconnected: other error: {e}");
                break;
            }
        }
    }

    // If we get here, the connection has been dropped, close our window.
    info!("Connection to Pipeweaver Lost, sending Close");
    let _ = tx.send(WindowMessage::Close);
}

fn ipc_thread_main(tx: mpsc::Sender<WindowMessage>) -> Result<()> {
    debug!("Spawning IPC Socket Handler");

    let socket_path = get_socket_file_path();
    if let Some(parent) = socket_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        warn!("Failed to create socket directory {parent:?}: {e}");
        bail!("Failed to Open IPC Socket");
    }

    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(e) => {
            warn!("Failed to bind to socket: {e}");
            bail!("Failed to bind to socket: {e}");
        }
    };

    listener.set_nonblocking(true)?;

    debug!("IPC listener started at {socket_path:?}");
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut msg = String::new();
                if let Err(e) = stream.read_to_string(&mut msg) {
                    warn!("Failed to read message from stream: {e}");
                    continue;
                }

                if msg == "TRIGGER" {
                    // Send a PING message to the Qt thread, this will return the next time
                    // Qt polls from the qml into the rust code. If the channel is dead, or  we
                    // exceed the timeout, assume the Qt side is dead.

                    let (ping_tx, ping_rx) = mpsc::sync_channel(1);
                    if tx.send(WindowMessage::Ping(ping_tx)).is_err() {
                        // Channel is broken — Qt side is gone.
                        warn!("Qt channel is broken, reporting instance as dead");
                        let _ = stream.write_all(b"DEAD");
                        continue;
                    }

                    match ping_rx.recv_timeout(QT_PING_TIMEOUT) {
                        Ok(()) => {
                            // Qt is alive — now forward the actual trigger.
                            let _ = tx.send(WindowMessage::Trigger);
                            if let Err(e) = raise_window() {
                                warn!("Failed to raise window: {e}");
                            }
                            let _ = stream.write_all(b"OK");
                        }
                        Err(_) => {
                            warn!("Qt failed to respond to ping, reporting instance as dead");
                            let _ = stream.write_all(b"DEAD");
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                warn!("Unexpected socket error: {e}");
                break;
            }
        }
    }
    let _ = fs::remove_file(&socket_path);
    debug!("IPC Socket closed (thread)");
    Ok(())
}

/// Returns true if a healthy existing instance handled the launch (we should exit).
/// Returns false if something is wrong, at which point we kill the existing process.
pub fn handle_active_instance() -> bool {
    let socket_path = get_socket_file_path();
    debug!("Looking for Socket at {socket_path:?}");

    if !socket_path.exists() {
        debug!("Existing socket is not present");
        return false;
    }

    debug!("Attempting to Connect to Existing Socket");
    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            debug!("Failed to Connect to Socket: {e}, removing stale socket file");
            let _ = fs::remove_file(&socket_path);
            return false;
        }
    };

    // Set a read timeout, if we don't get a response in this time, assume dead
    let _ = stream.set_read_timeout(Some(IPC_REPLY_TIMEOUT));

    // Attempt to write a trigger message..
    if let Err(e) = stream.write_all(b"TRIGGER") {
        debug!("Failed to write TRIGGER: {e}");
        kill_existing_instance();
        return false;
    }
    let _ = stream.shutdown(Shutdown::Write);

    // Check the reply, if we're OK everything is good, otherwise it ded.
    let mut reply = [0u8; 4];
    let response_size = stream.read(&mut reply).unwrap_or(0);

    if &reply[..response_size] == b"OK" {
        debug!("Existing instance is healthy");
        return true;
    }

    // "DEAD", timeout or zero response size, or anything unexpected — kill and take over.
    if response_size == 0 {
        debug!("No reply from existing instance (hung), killing it");
    } else {
        debug!("Existing instance reported itself as dead, killing it");
    }
    kill_existing_instance();
    false
}

fn kill_existing_instance() {
    let pid_path = get_pid_file_path();
    match fs::read_to_string(&pid_path) {
        Ok(contents) => {
            if let Ok(pid) = contents.trim().parse::<u32>() {
                debug!("Sending SIGTERM to stale instance with PID {pid}");
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
                // Give it a moment to clean up, then force if still alive.
                thread::sleep(Duration::from_millis(500));
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGKILL);
                }
            }
            let _ = fs::remove_file(&pid_path);
        }
        Err(e) => {
            warn!("Could not read PID file to kill stale instance: {e}");
        }
    }
    let _ = fs::remove_file(get_socket_file_path());
}

fn write_pid_file() {
    let pid = std::process::id();
    let path = get_pid_file_path();
    if let Err(e) = fs::write(&path, pid.to_string()) {
        warn!("Failed to write PID file at {path:?}: {e}");
    }
}

fn get_socket_file_path() -> PathBuf {
    let mut path = get_runtime_path().unwrap_or_else(|_| env::temp_dir());
    path.push(format!("{}.sock", APP_NAME));
    path
}

fn get_pid_file_path() -> PathBuf {
    let mut path = get_runtime_path().unwrap_or_else(|_| env::temp_dir());
    path.push(format!("{}.pid", APP_NAME));
    path
}

fn get_runtime_path() -> Result<PathBuf> {
    BaseDirs::new()
        .and_then(|base| base.runtime_dir().map(|p| p.to_path_buf()))
        .map(Ok::<PathBuf, Error>)
        .unwrap_or_else(|| {
            let tmp_dir = env::temp_dir().join(APP_NAME);
            if !tmp_dir.exists() {
                fs::create_dir_all(&tmp_dir)?;
            }
            Ok(tmp_dir)
        })
}

pub fn display_error(message: String) {
    use std::process::Command;
    if let Err(e) = Command::new("kdialog")
        .arg("--title")
        .arg("Pipeweaver UI")
        .arg("--error")
        .arg(message.clone())
        .output()
    {
        println!("Error Running kdialog: {e}, falling back to zenity..");
        let _ = Command::new("zenity")
            .arg("--title")
            .arg("Pipeweaver UI")
            .arg("--error")
            .arg("--text")
            .arg(message)
            .output();
    }
}
