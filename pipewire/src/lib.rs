pub extern crate oneshot;
mod default_device;
mod manager;
mod registry;
mod store;

use crate::manager::run_pw_main_loop;
use anyhow::{Result, anyhow, bail};
use log::{info, trace, warn};
use oneshot::TryRecvError;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;
use std::thread::{JoinHandle, sleep};
use std::time::{Duration, Instant};
use ulid::Ulid;

type PWSender = pipewire::channel::Sender<PipewireInternalMessage>;
type PWReceiver = pipewire::channel::Receiver<PipewireInternalMessage>;

type Sender = mpsc::Sender<PipewireInternalMessage>;
type Receiver = mpsc::Receiver<PipewireInternalMessage>;

#[derive(Debug)]
pub enum PipewireMessage {
    CreateDeviceNode(NodeProperties),
    CreateFilterNode(FilterProperties),
    CreateDeviceLink(LinkType, LinkType, oneshot::Sender<()>),

    RemoveDeviceNode(Ulid),
    RemoveFilterNode(Ulid),
    RemoveDeviceLink(LinkType, LinkType),

    GetFilterParameters(Ulid, oneshot::Sender<Result<Vec<FilterProperty>>>),
    SetFilterValue(Ulid, u32, FilterValue, oneshot::Sender<Result<String>>),

    SetNodeVolume(Ulid, u8),
    SetNodeMute(Ulid, bool),

    SetApplicationTarget(u32, Ulid),
    SetApplicationVolume(u32, u8),
    SetApplicationMute(u32, bool),
    ClearApplicationTarget(u32),

    DestroyUnmanagedLinks(u32),

    Quit,
}

pub enum PipewireInternalMessage {
    CreateDeviceNode(NodeProperties, oneshot::Sender<Result<()>>),
    CreateFilterNode(FilterProperties, oneshot::Sender<Result<()>>),
    CreateDeviceLink(
        LinkType,
        LinkType,
        oneshot::Sender<()>,
        oneshot::Sender<Result<()>>,
    ),

    RemoveDeviceNode(Ulid, oneshot::Sender<Result<()>>),
    RemoveFilterNode(Ulid, oneshot::Sender<Result<()>>),
    RemoveDeviceLink(LinkType, LinkType, oneshot::Sender<Result<()>>),

    GetFilterParameters(Ulid, oneshot::Sender<Result<Vec<FilterProperty>>>),
    SetFilterValue(Ulid, u32, FilterValue, oneshot::Sender<Result<String>>),

    SetNodeVolume(Ulid, u8, oneshot::Sender<Result<()>>),
    SetNodeMute(Ulid, bool, oneshot::Sender<Result<()>>),
    SetApplicationVolume(u32, u8, oneshot::Sender<Result<()>>),
    SetApplicationMute(u32, bool, oneshot::Sender<Result<()>>),

    SetApplicationTarget(u32, Ulid, oneshot::Sender<Result<()>>),
    ClearApplicationTarget(u32, oneshot::Sender<Result<()>>),

    DestroyUnmanagedLinks(u32, oneshot::Sender<Result<()>>),
    Quit(oneshot::Sender<Result<()>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipewireReceiver {
    Quit,

    AnnouncedClock(Option<u32>),

    DefaultChanged(MediaClass, NodeTarget),

    DeviceAdded(DeviceNode),
    DeviceRemoved(u32),
    DeviceUsable(u32, bool),

    ApplicationAdded(ApplicationNode),
    ApplicationTargetChanged(u32, Option<NodeTarget>),
    ApplicationTitleChanged(u32, String),
    ApplicationVolumeChanged(u32, u8),
    ApplicationMuteChanged(u32, bool),
    ApplicationRemoved(u32),

    NodeVolumeChanged(Ulid, u8),
    NodeMuteChanged(Ulid, bool),

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
        let start = Instant::now();
        trace!("Sending Message to Pipewire: {:?}", message);

        // Check if this is a message that handles its own response channel
        let uses_own_channel = matches!(
            message,
            PipewireMessage::GetFilterParameters(..) | PipewireMessage::SetFilterValue(..)
        );
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
            PipewireMessage::DestroyUnmanagedLinks(id) => {
                PipewireInternalMessage::DestroyUnmanagedLinks(id, tx)
            }
            PipewireMessage::GetFilterParameters(id, tx) => {
                PipewireInternalMessage::GetFilterParameters(id, tx)
            }
            PipewireMessage::SetFilterValue(id, prop, value, tx) => {
                PipewireInternalMessage::SetFilterValue(id, prop, value, tx)
            }
            PipewireMessage::SetNodeVolume(id, volume) => {
                PipewireInternalMessage::SetNodeVolume(id, volume, tx)
            }
            PipewireMessage::SetNodeMute(id, muted) => {
                PipewireInternalMessage::SetNodeMute(id, muted, tx)
            }
            PipewireMessage::SetApplicationTarget(app_id, target) => {
                PipewireInternalMessage::SetApplicationTarget(app_id, target, tx)
            }
            PipewireMessage::SetApplicationVolume(id, volume) => {
                PipewireInternalMessage::SetApplicationVolume(id, volume, tx)
            }
            PipewireMessage::SetApplicationMute(id, state) => {
                PipewireInternalMessage::SetApplicationMute(id, state, tx)
            }
            PipewireMessage::ClearApplicationTarget(id) => {
                PipewireInternalMessage::ClearApplicationTarget(id, tx)
            }
            PipewireMessage::Quit => PipewireInternalMessage::Quit(tx),
        };

        self.message_sender
            .send(message)
            .map_err(|e| anyhow!("Unable to Send Message: {}", e))?;

        // Only wait for response if the message doesn't handle its own channel
        if !uses_own_channel {
            let resp = rx.recv().map_err(|e| anyhow!("Error: {}", e))?;
            let stop = start.elapsed().as_millis();

            trace!("Received Response: {:?} in {}ms", resp, stop);
            resp
        } else {
            let stop = start.elapsed().as_millis();
            trace!("Message sent (uses own response channel) in {}ms", stop);
            Ok(())
        }
    }
}

impl Drop for PipewireRunner {
    fn drop(&mut self) {
        info!("[PIPEWIRE] Stopping");
        // Send an exit message
        let _ = self.send_message(PipewireMessage::Quit);

        // Wait on the threads to exit..
        if let Some(pipewire_thread) = self.pipewire_thread.take()
            && let Err(e) = pipewire_thread.join()
        {
            warn!("Unable to Join Pipewire Thread: {:?}", e);
        }

        info!("[PIPEWIRE] Main Thread Stopped");

        if let Some(messaging_thread) = self.messaging_thread.take()
            && let Err(e) = messaging_thread.join()
        {
            warn!("Unable to Join Message Thread: {:?}", e);
        }

        info!("[PIPEWIRE] Message Thread Stopped");
        info!("[PIPEWIRE] Stopped");
    }
}

// Maps messages from an mpsc::channel to a pipewire::channel
fn run_message_loop(receiver: Receiver, sender: PWSender) {
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

#[derive(Debug)]
pub struct NodeProperties {
    pub node_id: Ulid,

    // Node specific variables..
    pub node_name: String,
    pub node_nick: String,
    pub node_description: String,

    // Setup
    pub initial_volume: u8,

    // App specific variables..
    pub app_id: String,
    pub app_name: String,

    // Node Configuration
    pub linger: bool,
    pub class: MediaClass,
    pub managed_volume: bool,

    // Latency Configuration
    pub buffer: u32,
    pub rate: u32,

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

    pub ready_sender: Option<oneshot::Sender<()>>,
}
impl Debug for FilterProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterProperties")
            .field("filter_id", &self.filter_id)
            .field("filter_name", &self.filter_name)
            .field("filter_nick", &self.filter_nick)
            .field("filter_description", &self.filter_description)
            .field("app_id", &self.app_id)
            .field("app_name", &self.app_name)
            .field("class", &self.class)
            .field("linger", &self.linger)
            .finish()
    }
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NodeTarget {
    Node(Ulid),
    UnmanagedNode(u32),
}

pub type FilterCallback = dyn FnMut(Vec<&mut [f32]>, Vec<&mut [f32]>) + Send;
pub trait FilterHandler: Send + 'static {
    fn get_properties(&self) -> Vec<FilterProperty>;
    fn get_property(&self, id: u32) -> FilterProperty;
    fn set_property(&mut self, id: u32, value: FilterValue) -> Result<String>;

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
    Bool(bool),
    Enum(String, u32),
}

#[derive(Debug)]
pub struct FilterProperty {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    pub value: FilterValue,

    pub min: f32,
    pub max: f32,

    pub enum_def: Option<HashMap<u32, String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceNode {
    pub node_id: u32,
    pub node_class: MediaClass,

    pub is_usable: bool,

    pub name: Option<String>,
    pub nickname: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationNode {
    pub node_id: u32,
    pub node_class: MediaClass,
    pub media_target: Option<Option<NodeTarget>>,

    pub volume: u8,
    pub muted: bool,

    pub title: Option<String>,

    pub process_name: String,
    pub name: String,
}
