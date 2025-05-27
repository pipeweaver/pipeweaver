pub extern crate oneshot;
mod manager;
mod registry;
mod store;

use crate::manager::run_pw_main_loop;
use anyhow::{anyhow, bail, Result};
use log::{debug, info, warn};
use oneshot::TryRecvError;
use std::sync::mpsc;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use ulid::Ulid;

type PWSender = pipewire::channel::Sender<PipewireMessage>;
type PWInternalSender = pipewire::channel::Sender<PipewireInternalMessage>;
type PWReceiver = pipewire::channel::Receiver<PipewireInternalMessage>;

type Sender = mpsc::Sender<PipewireInternalMessage>;
type Receiver = mpsc::Receiver<PipewireInternalMessage>;

pub enum PipewireMessage {
    CreateDeviceNode(NodeProperties),
    CreateFilterNode(FilterProperties),
    CreateDeviceLink(LinkType, LinkType, Option<oneshot::Sender<()>>),

    RemoveDeviceNode(Ulid),
    RemoveFilterNode(Ulid),
    RemoveDeviceLink(LinkType, LinkType),

    SetFilterValue(Ulid, u32, FilterValue),

    Quit,
}

pub enum PipewireInternalMessage {
    CreateDeviceNode(NodeProperties, oneshot::Sender<Result<()>>),
    CreateFilterNode(FilterProperties, oneshot::Sender<Result<()>>),
    CreateDeviceLink(
        LinkType,
        LinkType,
        Option<oneshot::Sender<()>>,
        oneshot::Sender<Result<()>>,
    ),

    RemoveDeviceNode(Ulid, oneshot::Sender<Result<()>>),
    RemoveFilterNode(Ulid, oneshot::Sender<Result<()>>),
    RemoveDeviceLink(
        LinkType,
        LinkType,
        oneshot::Sender<Result<()>>,
    ),

    SetFilterValue(Ulid, u32, FilterValue, oneshot::Sender<Result<()>>),
    Quit(oneshot::Sender<Result<()>>),
}

#[derive(Debug, PartialEq)]
pub enum PipewireReceiver {
    Quit,

    DeviceAdded(PipewireNode),
    DeviceRemoved(u32),

    ManagedLinkDropped(LinkType, LinkType),
}

pub struct NamingScheme {
    pub app_id: String,
    pub app_name: String,
    pub group_prefix: String,
}

// We'll use Options on the thread handles, so we can take them during drop
pub struct PipewireRunner {
    pipewire_thread: Option<JoinHandle<()>>,
    messaging_thread: Option<JoinHandle<()>>,
    message_sender: Sender,
}

impl PipewireRunner {
    pub fn new(callback_tx: mpsc::Sender<PipewireReceiver>) -> Result<Self> {
        // First, we need our pipewire messaging queue, so establish that here
        let (pw_tx, pw_rx) = pipewire::channel::channel();
        let (tx, rx) = mpsc::channel();

        // This channel lets us call back once the pipewire code is ready
        let (start_tx, start_rx) = oneshot::channel();

        // Next, spawn up the pipewire mainloop in a separate thread
        let pipewire_handle = thread::spawn(|| run_pw_main_loop(pw_rx, start_tx, callback_tx));

        // Await a response from that thread to indicate we're ready to handle messages
        loop {
            match start_rx.try_recv() {
                Ok(Ok(())) => {
                    break;
                }
                Ok(Err(error)) => {
                    warn!("Error Starting Pipewire Manager: {}", error);
                    bail!(error.to_string());
                }
                Err(e) => {
                    if e == TryRecvError::Disconnected {
                        panic!("Channel Closed before Response");
                    }
                    sleep(Duration::from_millis(5));
                }
            }
        }

        let message_handle = thread::spawn(|| run_message_loop(rx, pw_tx));

        Ok(Self {
            pipewire_thread: Some(pipewire_handle),
            messaging_thread: Some(message_handle),
            message_sender: tx,
        })
    }

    pub fn send_message(&self, message: PipewireMessage) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        let message = match message {
            PipewireMessage::CreateDeviceNode(n) => {
                PipewireInternalMessage::CreateDeviceNode(n, tx)
            }
            PipewireMessage::CreateFilterNode(f) => {
                PipewireInternalMessage::CreateFilterNode(f, tx)
            }
            PipewireMessage::CreateDeviceLink(lt, lt2, cb) => {
                PipewireInternalMessage::CreateDeviceLink(lt, lt2, cb, tx)
            }
            PipewireMessage::RemoveDeviceNode(id) => {
                PipewireInternalMessage::RemoveDeviceNode(id, tx)
            }
            PipewireMessage::RemoveFilterNode(id) => {
                PipewireInternalMessage::RemoveFilterNode(id, tx)
            }
            PipewireMessage::RemoveDeviceLink(lt, lt2) => {
                PipewireInternalMessage::RemoveDeviceLink(lt, lt2, tx)
            }
            PipewireMessage::SetFilterValue(id, prop, value) => {
                PipewireInternalMessage::SetFilterValue(id, prop, value, tx)
            }
            PipewireMessage::Quit => PipewireInternalMessage::Quit(tx),
        };

        self.message_sender
            .send(message)
            .map_err(|e| anyhow!("Unable to Send Message: {}", e))?;

        rx.recv().map_err(|e| anyhow!("Error: {}", e))?
    }
}

impl Drop for PipewireRunner {
    fn drop(&mut self) {
        info!("[PIPEWIRE] Stopping");
        // Send an exit message
        let _ = self.send_message(PipewireMessage::Quit);

        // Wait on the threads to exit..
        if let Some(pipewire_thread) = self.pipewire_thread.take() {
            if let Err(e) = pipewire_thread.join() {
                warn!("Unable to Join Pipewire Thread: {:?}", e);
            }
        }
        info!("[PIPEWIRE] Main Thread Stopped");

        if let Some(messaging_thread) = self.messaging_thread.take() {
            if let Err(e) = messaging_thread.join() {
                warn!("Unable to Join Message Thread: {:?}", e);
            }
        }
        info!("[PIPEWIRE] Message Thread Stopped");
        info!("[PIPEWIRE] Stopped");
    }
}

// Maps messages from an mpsc::channel to a pipewire::channel
fn run_message_loop(receiver: Receiver, sender: PWInternalSender) {
    loop {
        match receiver.recv() {
            Ok(PipewireInternalMessage::Quit(incoming_tx)) => {
                let (tx, rx) = oneshot::channel();
                let _ = sender.send(PipewireInternalMessage::Quit(tx));
                let _ = rx.recv();
                let _ = incoming_tx.send(Ok(()));
                break;
            }
            Ok(message) => {
                let _ = sender.send(message);
            }
            Err(_) => {
                let (tx, rx) = oneshot::channel();
                let _ = sender.send(PipewireInternalMessage::Quit(tx));
                let _ = rx.recv();
                break;
            }
        }
    }
    info!("[PW-LIB] Message Loop Stopped");
}

pub struct NodeProperties {
    pub node_id: Ulid,

    // Node specific variables..
    pub node_name: String,
    pub node_nick: String,
    pub node_description: String,

    // App specific variables..
    pub app_id: String,
    pub app_name: String,

    // Node Configuration
    pub linger: bool,
    pub class: MediaClass,

    // Latency Configuration
    pub buffer: u32,

    // Ready Sender
    pub ready_sender: Option<oneshot::Sender<()>>,
}

pub struct FilterProperties {
    pub filter_id: Ulid,

    pub filter_name: String,
    pub filter_nick: String,
    pub filter_description: String,

    pub app_id: String,
    pub app_name: String,

    pub class: MediaClass,
    pub linger: bool,
    pub callback: Box<dyn FilterHandler>,

    pub receive_only: bool,
    pub ready_sender: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MediaClass {
    Source,
    Sink,
    Duplex,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LinkType {
    Node(Ulid),
    Filter(Ulid),
    UnmanagedNode(u32),
}

pub type FilterCallback = dyn FnMut(Vec<&mut [f32]>, Vec<&mut [f32]>) + Send;
pub trait FilterHandler: Send + 'static {
    fn get_properties(&self) -> Vec<FilterProperty>;
    fn get_property(&self, id: u32) -> FilterProperty;
    fn set_property(&mut self, id: u32, value: FilterValue);

    fn process_samples(&mut self, inputs: Vec<&mut [f32]>, outputs: Vec<&mut [f32]>);
}

// We need these because while *WE* know what values are coming in and out, rust doesn't
// so gives us a wrapper around some common types that can be read out appropriately by the filter
#[derive(Debug)]
pub enum FilterValue {
    Int32(i32),
    Float32(f32),
    UInt8(u8),
    UInt32(u32),
    String(String),
}

pub struct FilterProperty {
    pub id: u32,
    pub name: String,
    pub value: FilterValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipewireNode {
    pub node_id: u32,
    pub node_class: MediaClass,

    pub name: Option<String>,
    pub nickname: Option<String>,
    pub description: Option<String>,
}
