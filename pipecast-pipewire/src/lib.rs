pub extern crate tokio;
pub extern crate ulid;
mod store;
mod manager;
mod registry;

use crate::manager::run_pw_main_loop;
use anyhow::{anyhow, bail, Result};
use log::{info, warn};
use std::sync::mpsc;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use tokio::sync::oneshot;
use ulid::Ulid;

type PWSender = pipewire::channel::Sender<PipewireMessage>;
type PWReceiver = pipewire::channel::Receiver<PipewireMessage>;

type Sender = mpsc::Sender<PipewireMessage>;
type Receiver = mpsc::Receiver<PipewireMessage>;

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
        let (start_tx, mut start_rx) = oneshot::channel();

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
        self.message_sender.send(message).map_err(|e| anyhow!("Unable to Send Message: {}", e))
    }
}

impl Drop for PipewireRunner {
    fn drop(&mut self) {
        // Send an exit message
        let _ = self.message_sender.send(PipewireMessage::Quit);

        // Wait on the threads to exit..
        if let Some(pipewire_thread) = self.pipewire_thread.take() {
            if let Err(e) = pipewire_thread.join() {
                warn!("Unable to Join Pipewire Thread: {:?}", e);
            }
        }
        if let Some(messaging_thread) = self.messaging_thread.take() {
            if let Err(e) = messaging_thread.join() {
                warn!("UNable to Join Message Thread: {:?}", e);
            }
        }

        info!("Pipewire Manager Stopped");
    }
}

// Maps messages from an mpsc::channel to a pipewire::channel
fn run_message_loop(receiver: Receiver, sender: PWSender) {
    loop {
        match receiver.recv().unwrap_or(PipewireMessage::Quit) {
            PipewireMessage::Quit => {
                // Send this message to pipewire
                let _ = sender.send(PipewireMessage::Quit);
                break;
            }
            message => {
                let _ = sender.send(message);
            }
        }
    }
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