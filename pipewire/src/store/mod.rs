//! `Store` is the single source of truth for everything the pipewire thread
//! knows about: the objects we manage ourselves (nodes/filters/links we
//! created) and the objects pipewire told us about that we don't own
//! (devices, clients, and their nodes/links). Because that's a lot of
//! ground to cover, the behaviour is split across submodules by concern
//! rather than living in one file:
//!
//! - [`types`] - the plain data records `Store` holds (`NodeStore`, `FilterStore`, ...)
//! - [`defaults`] - tracking/announcing the default sink & source
//! - [`nodes`] - lifecycle of nodes we manage (create -> ready -> remove), volume/mute
//! - [`filters`] - lifecycle of filters we manage
//! - [`links`] - lifecycle of links we manage, including the pending-sync dance
//! - [`unmanaged`] - lifecycle of devices/clients/links we don't manage
//! - [`remove`] - the generic "something with this id went away" dispatcher
//! - [`utils`] - small helpers shared by more than one submodule
//!
//! All of `Store`'s fields are private to this module, but Rust's privacy
//! rules mean every submodule listed above (being a descendant of `store`)
//! can still reach them directly. External code only ever sees the `pub`
//! surface re-exported below.

mod defaults;
mod filters;
mod links;
mod nodes;
mod remove;
mod types;
mod unmanaged;

pub(crate) mod utils;

pub(crate) use types::TargetType;
pub use types::{FilterStore, LinkStore, LinkStoreMap, NodeStoreState, PendingLinkSync};
pub(crate) use types::{NodeStore, PortLocation};

use crate::PipewireReceiver;
use crate::default_device::DefaultDevice;
use crate::registry::client::RegistryClient;
use crate::registry::client_node::RegistryClientNode;
use crate::registry::device::RegistryDevice;
use crate::registry::device_node::RegistryDeviceNode;
use crate::registry::factory::RegistryFactory;
use crate::registry::link::RegistryLink;
use crate::registry::metadata::MetadataStore;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::mpsc;
use ulid::Ulid;

pub(crate) struct Store {
    // The main Session Proxy Metadata
    session_proxy: Option<MetadataStore>,
    settings_proxy: Option<MetadataStore>,

    // Pipewire Factories, helps us track types
    factories: HashMap<u32, RegistryFactory>,

    // The default Sink / Source
    default_sink: DefaultDevice,   // (eg pipeweaver_system)
    default_source: DefaultDevice, // (eg pipeweaver_chat_mic)

    // These are nodes, filters and links created by us
    managed_nodes: HashMap<Ulid, NodeStore>,
    managed_filters: HashMap<Ulid, FilterStore>,
    managed_links: HashMap<Ulid, LinkStore>,

    // These are devices and device nodes not created by us
    pub(crate) unmanaged_devices: HashMap<u32, RegistryDevice>,
    pub(crate) unmanaged_device_nodes: HashMap<u32, RegistryDeviceNode>,

    // These are clients and client nodes not created by us
    unmanaged_clients: HashMap<u32, RegistryClient>,
    unmanaged_client_nodes: HashMap<u32, RegistryClientNode>,

    // These are links found which aren't specifically between managed targets
    unmanaged_links: HashMap<u32, RegistryLink>,

    // Usable Nodes are unmanaged device / client nodes with a stereo setup
    usable_client_nodes: Vec<u32>,

    // Pending Stuff
    pub(crate) pending_link_syncs: Vec<PendingLinkSync>,
    pub(crate) pending_device_syncs: HashMap<i32, u32>,
    pub(crate) pending_filter_syncs: HashMap<i32, Ulid>,

    callback_tx: mpsc::Sender<PipewireReceiver>,
}

impl Store {
    pub fn new(callback_tx: mpsc::Sender<PipewireReceiver>) -> Self {
        Self {
            session_proxy: None,
            settings_proxy: None,

            factories: HashMap::new(),

            default_sink: DefaultDevice::default(),
            default_source: DefaultDevice::default(),

            managed_nodes: HashMap::new(),
            managed_filters: HashMap::new(),
            managed_links: HashMap::new(),

            unmanaged_devices: HashMap::new(),
            unmanaged_device_nodes: HashMap::new(),

            unmanaged_clients: HashMap::new(),
            unmanaged_client_nodes: HashMap::new(),

            unmanaged_links: HashMap::new(),

            pending_link_syncs: vec![],
            pending_device_syncs: HashMap::new(),
            pending_filter_syncs: HashMap::new(),

            usable_client_nodes: vec![],

            callback_tx,
        }
    }

    // ----- SESSION HANDLING -----
    pub fn set_session_proxy(&mut self, session: MetadataStore) {
        if self.session_proxy.is_some() {
            warn!("Attempting to redefine default Session Manager, aborting.");
            return;
        }
        info!("Session Proxy Found");
        self.session_proxy = Some(session);
    }

    pub fn set_settings_proxy(&mut self, settings: MetadataStore) {
        if self.settings_proxy.is_some() {
            warn!("Attempting to redefine default Settings Manager, aborting.");
            return;
        }
        info!("Settings Proxy Found");
        self.settings_proxy = Some(settings);
    }

    pub fn announce_clock_rate(&self, rate: Option<u32>) {
        let _ = self
            .callback_tx
            .send(PipewireReceiver::AnnouncedClock(rate));
    }

    // ----- FACTORIES -----
    pub fn factory_add(&mut self, id: u32, factory: RegistryFactory) {
        self.factories.insert(id, factory);
    }

    #[allow(unused)]
    pub fn factory_get(&self, id: u32) -> Option<&RegistryFactory> {
        self.factories.get(&id)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        debug!("Dropping Pipewire Store");
    }
}
