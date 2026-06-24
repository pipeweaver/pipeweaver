pub mod client;
pub(crate) mod client_node;
pub(crate) mod device;
pub(crate) mod device_node;
pub(crate) mod factory;
pub(crate) mod link;
pub(crate) mod metadata;
pub(crate) mod port;

use crate::registry::client::handle_client;
use crate::registry::client_node::handle_client_node;
use crate::registry::device::handle_device;
use crate::registry::device_node::handle_device_node;
use crate::registry::factory::handle_factory;
use crate::registry::link::handle_link;
use crate::registry::metadata::handle_metadata;
use crate::registry::port::handle_port;
use crate::store::Store;

use log::debug;
use pipewire::core::Core;

use pipewire::registry::Listener;
use pipewire::registry::Registry;

use pipewire::keys::{MEDIA_CLASS, OBJECT_SERIAL};
use pipewire::types::ObjectType;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct PipewireRegistry {
    registry: Rc<RefCell<Registry>>,
    store: Rc<RefCell<Store>>,
    core: Rc<Core>,

    // These two need to exist, if the Listeners are dropped they simply stop working.
    registry_listener: Option<Listener>,
    registry_removal_listener: Option<Listener>,
}

impl PipewireRegistry {
    pub fn new(registry: Registry, store: Rc<RefCell<Store>>, core: Rc<Core>) -> Self {
        let mut registry = Self {
            registry: Rc::new(RefCell::new(registry)),
            store,
            core,
            registry_listener: None,
            registry_removal_listener: None,
        };

        registry.registry_listener = Some(registry.register_listener());
        registry.registry_removal_listener = Some(registry.registry_removal_listener());

        registry
    }

    pub fn register_listener(&self) -> Listener {
        let local_store = Rc::downgrade(&self.store);
        let listener_store = Rc::downgrade(&self.store);
        let registry = self.registry.clone();
        let core = self.core.clone();

        self.registry
            .borrow()
            .add_listener_local()
            .global(move |global| {
                let id = global.id;

                // If the store has been dropped we can early-return
                let Some(local_store) = local_store.upgrade() else {
                    return;
                };

                let mut store = local_store.borrow_mut();
                match global.type_ {
                    ObjectType::Device => {
                        let reg = registry.clone();
                        let inner_store = listener_store.clone();
                        handle_device(id, global, reg, &mut store, inner_store);
                    }

                    ObjectType::Node => {
                        if let Some(props) = global.props {
                            // If we're receiving properties for a managed node, we just need to update
                            // the internal serial number if it's present.
                            if store.is_managed_node(id) {
                                if let Some(serial) = props
                                    .get(*OBJECT_SERIAL)
                                    .and_then(|s| s.parse::<u32>().ok())
                                {
                                    store.managed_node_set_pw_serial(id, serial);
                                }
                                return;
                            }

                            let reg = registry.clone();
                            let store = &mut store;
                            let inner_store = listener_store.clone();
                            let core = core.clone();
                            match props.get(*MEDIA_CLASS) {
                                Some("Audio/Sink") | Some("Audio/Source") => {
                                    handle_device_node(id, core, global, reg, store, inner_store);
                                }
                                Some("Stream/Input/Audio") | Some("Stream/Output/Audio") => {
                                    handle_client_node(id, global, reg, store, inner_store);
                                }
                                _ => {}
                            }
                        }
                    }

                    ObjectType::Port => handle_port(id, global, &mut store),
                    ObjectType::Link => handle_link(id, global, &mut store),
                    ObjectType::Factory => handle_factory(id, global, &mut store),

                    ObjectType::Client => {
                        let reg = registry.clone();
                        handle_client(id, global, reg, &mut store, listener_store.clone())
                    }

                    ObjectType::Metadata => {
                        let reg = registry.clone();
                        handle_metadata(global, reg, &mut store, listener_store.clone());
                    }

                    _ => {}
                }
            })
            .register()
    }

    pub fn registry_removal_listener(&self) -> Listener {
        let store = Rc::downgrade(&self.store);
        self.registry
            .borrow()
            .add_listener_local()
            .global_remove(move |id| {
                if let Some(store) = store.upgrade() {
                    store.borrow_mut().remove_by_id(id);
                }
            })
            .register()
    }

    pub fn destroy_global(&self, id: u32) {
        self.registry.borrow().destroy_global(id);
    }
}

// pipewire-rs doesn't seem to provide one of these, it does have from_str and to_str, but they're
// crate public, so we can't use them, and they're only looking for the last chunk.
pub(crate) fn to_object_type(input: &str) -> ObjectType {
    match input {
        "PipeWire:Interface:Client" => ObjectType::Client,
        "PipeWire:Interface:ClientEndpoint" => ObjectType::ClientEndpoint,
        "PipeWire:Interface:ClientNode" => ObjectType::ClientNode,
        "PipeWire:Interface:ClientSession" => ObjectType::ClientSession,
        "PipeWire:Interface:Core" => ObjectType::Core,
        "PipeWire:Interface:Device" => ObjectType::Device,
        "PipeWire:Interface:Endpoint" => ObjectType::Endpoint,
        "PipeWire:Interface:EndpointLink" => ObjectType::EndpointLink,
        "PipeWire:Interface:EndpointStream" => ObjectType::EndpointStream,
        "PipeWire:Interface:Factory" => ObjectType::Factory,
        "PipeWire:Interface:Link" => ObjectType::Link,
        "PipeWire:Interface:Metadata" => ObjectType::Metadata,
        "PipeWire:Interface:Module" => ObjectType::Module,
        "PipeWire:Interface:Node" => ObjectType::Node,
        "PipeWire:Interface:Port" => ObjectType::Port,
        "PipeWire:Interface:Profiler" => ObjectType::Profiler,
        "PipeWire:Interface:Registry" => ObjectType::Registry,
        "PipeWire:Interface:Session" => ObjectType::Session,
        _ => ObjectType::Other(input.to_string()),
    }
}

impl Drop for PipewireRegistry {
    fn drop(&mut self) {
        debug!("Dropping Pipewire Registry");
    }
}
