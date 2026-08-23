use crate::registry::PipewireRegistry;
use crate::store::{Store, create_port_link};
use crate::{
    FilterHandler, FilterProperties, FilterProperty, FilterValue, LinkType, NodeProperties,
    NodeTarget, PipewireInternalMessage, PipewireReceiver,
};
use crate::{MediaClass, PWReceiver};
use anyhow::Result;
use anyhow::{anyhow, bail};
use log::{debug, error, info};
use pipewire::core::{CoreRc, Listener};
use pipewire::keys::MEDIA_CATEGORY;
use pipewire::properties::properties;
use pipewire::registry::RegistryRc;

use oneshot::Sender;

use pipewire::context;
use pipewire::main_loop::MainLoopRc;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use ulid::Ulid;

pub(crate) struct FilterData {
    pub callback: Box<dyn FilterHandler>,
}

struct PipewireManager {
    core: CoreRc,
    registry: PipewireRegistry,

    store: Rc<RefCell<Store>>,
    mainloop: MainLoopRc,

    _core_listener: Option<Listener>,
}

impl PipewireManager {
    pub fn new(
        core: CoreRc,
        mainloop: MainLoopRc,
        registry: RegistryRc,
        callback_tx: mpsc::Sender<PipewireReceiver>,
    ) -> Self {
        let store = Rc::new(RefCell::new(Store::new(callback_tx.clone())));
        let registry = PipewireRegistry::new(registry, store.clone(), core.clone());

        Self {
            core,
            registry,
            store,

            mainloop,
            _core_listener: None,
        }
    }

    pub fn create_core_listener(this: &Rc<RefCell<Self>>) {
        let (core, store, mainloop) = {
            let this_ref = this.borrow();
            (
                this_ref.core.clone(),
                this_ref.store.clone(),
                this_ref.mainloop.clone(),
            )
        };

        let done_store = Rc::downgrade(&store);
        let done_mainloop = mainloop.downgrade();
        let done_core = core.clone();
        let done_link_store = Rc::downgrade(&store);

        let core_listener = core
            .add_listener_local()
            .done(move |_id, seq| {
                let Some(store_rc) = done_store.upgrade() else {
                    return;
                };

                let mut store_ref = store_rc.borrow_mut();

                // Pending Links
                if let Some(parent) = store_ref.get_pending_link_parent_id_by_seq(seq.raw())
                    && let Some(link_id) = store_ref.get_next_pending_link(seq.raw())
                {
                    debug!("Attempting to Create next Link: {}", parent);
                    let _ = create_port_link(&done_core, parent, link_id, done_link_store.clone());
                    return;
                }

                // --- Device sync ---
                if let Some(node_id) = store_ref.pending_device_syncs.remove(&seq.raw()) {
                    drop(store_ref);

                    // While in theory hitting this point, we SHOULD be completely ready and settled.
                    // In practice if we start linking up too quickly, we can experience some badness
                    // on the link. So provide a moment to settle.
                    if let Some(mainloop) = done_mainloop.upgrade() {
                        let timer_store = Rc::downgrade(&store_rc);
                        let timer = mainloop.loop_().add_timer(move |_| {
                            if let Some(store) = timer_store.upgrade() {
                                let mut store = store.borrow_mut();

                                if let Some(node) = store.unmanaged_device_nodes.get_mut(&node_id) {
                                    node.is_synced = true;
                                }

                                store.unmanaged_node_port_check(node_id);
                            }
                        });

                        timer.update_timer(Some(Duration::from_secs(1)), None);
                        std::mem::forget(timer);
                    }
                }
            })
            .register();

        this.borrow_mut()._core_listener = Some(core_listener);
    }

    pub fn create_node(&mut self, properties: NodeProperties) -> Result<()> {
        let listener_store = Rc::downgrade(&self.store);
        self.store
            .borrow_mut()
            .create_node(&self.core, properties, listener_store)
    }

    pub fn remove_node(&mut self, id: Ulid) -> Result<()> {
        self.store.borrow_mut().managed_node_remove(id);
        Ok(())
    }

    pub fn create_filter(&mut self, props: FilterProperties) -> Result<()> {
        let listener_store = Rc::downgrade(&self.store);
        self.store
            .borrow_mut()
            .create_filter(&self.core, props, listener_store)
    }

    pub fn remove_filter(&mut self, id: Ulid) -> Result<()> {
        self.store.borrow_mut().managed_filter_remove(id);
        Ok(())
    }

    pub fn get_filter_values(&mut self, id: Ulid) -> Result<Vec<FilterProperty>> {
        self.store.borrow().managed_filter_get_parameters(id)
    }

    pub fn set_filter_value(&mut self, id: Ulid, key: u32, value: FilterValue) -> Result<String> {
        // We need to grab the filter from the store, and pass the value set..
        self.store
            .borrow_mut()
            .managed_filter_set_parameter(id, key, value)
    }

    pub fn create_link(
        &mut self,
        source: LinkType,
        dest: LinkType,
        sender: Sender<()>,
    ) -> Result<()> {
        let listener_store = Rc::downgrade(&self.store);
        self.store.borrow_mut().create_link(
            &self.core,
            self.registry.raw(),
            source,
            dest,
            sender,
            listener_store,
        )
    }

    pub fn remove_link(&mut self, source: LinkType, destination: LinkType) -> Result<()> {
        self.store
            .borrow_mut()
            .managed_link_remove(&source, &destination);
        Ok(())
    }

    pub fn remove_all_unmanaged_links(&mut self, node: u32) -> Result<()> {
        for (&id, link) in self.store.borrow().get_unmanaged_links() {
            if link.input_node == node || link.output_node == node {
                self.registry.destroy_global(id);
            }
        }

        Ok(())
    }

    fn set_application_target(&mut self, app_id: u32, target: Ulid) -> Result<()> {
        let (pw_id, object_serial) = {
            let store = self.store.borrow();
            if let Some(target) = store.managed_node_get(target) {
                (target.pw_id, target.object_serial)
            } else {
                bail!("Target Not Found");
            }
        };

        let mut store = self.store.borrow_mut();
        if let Some(pw_id) = pw_id {
            store.unmanaged_node_set_meta(
                app_id,
                String::from("target.node"),
                Some(String::from("Spa:Id")),
                Some(pw_id.to_string()),
            );
        }
        if let Some(serial) = object_serial {
            store.unmanaged_node_set_meta(
                app_id,
                String::from("target.object"),
                Some(String::from("Spa:Id")),
                Some(serial.to_string()),
            );
        }

        Ok(())
    }

    fn clear_application_target(&mut self, app_id: u32) -> Result<()> {
        let mut store = self.store.borrow_mut();

        // This should (in theory) route a target to the default
        store.unmanaged_node_set_meta(
            app_id,
            String::from("target.node"),
            Some(String::from("Spa:Id")),
            Some("-1".to_string()),
        );
        store.unmanaged_node_set_meta(
            app_id,
            String::from("target.object"),
            Some(String::from("Spa:Id")),
            Some("-1".to_string()),
        );
        Ok(())
    }

    fn set_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        self.store.borrow_mut().set_volume(id, volume)
    }

    fn set_application_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        self.store.borrow_mut().set_application_volume(id, volume)
    }

    fn set_application_muted(&mut self, id: u32, state: bool) -> Result<()> {
        self.store.borrow_mut().set_application_muted(id, state)
    }

    fn set_device_volume(&mut self, id: u32, volume: u8) -> Result<()> {
        self.store
            .borrow_mut()
            .unmanaged_node_set_volume(id, volume)
    }
    fn set_device_muted(&mut self, id: u32, muted: bool) -> Result<()> {
        self.store.borrow_mut().unmanaged_node_set_mute(id, muted)
    }

    fn set_default_device(&mut self, class: MediaClass, node: NodeTarget) -> Result<()> {
        match class {
            MediaClass::Source => self.store.borrow_mut().set_default_source_node(node)?,
            MediaClass::Sink => self.store.borrow_mut().set_default_sink_node(node)?,
            MediaClass::Duplex => bail!("Can't set defaults on Duplex!"),
        }
        Ok(())
    }

    fn set_node_mute(&mut self, id: Ulid, mute: bool) -> Result<()> {
        self.store.borrow_mut().set_mute(id, mute)
    }
}

impl Drop for PipewireManager {
    fn drop(&mut self) {
        debug!("Dropping Pipewire Manager, cleaning up resources");
    }
}

pub fn run_pw_main_loop(
    pw_rx: PWReceiver,
    start_tx: oneshot::Sender<Result<()>>,
    callback_tx: mpsc::Sender<PipewireReceiver>,
) {
    debug!("Initialising Pipewire..");

    let Ok(mainloop) = MainLoopRc::new(None) else {
        start_tx
            .send(Err(anyhow!("Unable to create MainLoop")))
            .expect("OneShot Channel is broken!");
        return;
    };
    let Ok(context) = context::ContextRc::new(&mainloop, None) else {
        start_tx
            .send(Err(anyhow!("Unable to create Context")))
            .expect("OneShot Channel is broken!");
        return;
    };

    // Now we create a core, and flag it as a manager
    let Ok(core) = context.connect_rc(Some(properties!(
        *MEDIA_CATEGORY => "Manager",
    ))) else {
        start_tx
            .send(Err(anyhow!("Unable to Fetch Core from Context")))
            .expect("OneShot Channel is broken!");
        return;
    };

    let mainloop_error = mainloop.clone();
    let _core_listener = core
        .add_listener_local()
        .info(|info| {
            info!(
                "[PipeWire] Core Info: Name: {}, Version: {}, User Name: {}, Host Name: {}",
                info.name(),
                info.version(),
                info.user_name(),
                info.host_name()
            );
        })
        .error(move |id, _seq, res, msg| {
            if id == 0 {
                if res == -2 {
                    // -ENOENT: stale proxy race condition, safe to ignore
                    debug!("[PipeWire] Stale proxy: {}", msg);
                } else {
                    error!(
                        "[PipeWire] Core error (res={}): {}, shutting down",
                        res, msg
                    );
                    mainloop_error.quit();
                }
            }
        })
        .register();

    let Ok(registry) = core.get_registry_rc() else {
        start_tx
            .send(Err(anyhow!("Unable to Fetch Registry from Core")))
            .expect("OneShot Channel is broken!");
        return;
    };

    let manager = Rc::new(RefCell::new(PipewireManager::new(
        core,
        mainloop.clone(),
        registry,
        callback_tx.clone(),
    )));
    PipewireManager::create_core_listener(&manager);

    let receiver_clone = mainloop.clone();
    let _receiver = pw_rx.attach(mainloop.loop_(), {
        move |message| match message {
            PipewireInternalMessage::Quit(_, result) => {
                debug!("[PipeWire] Triggering Main Loop Quit");
                let _ = result.send(Ok(()));
                receiver_clone.quit();
            }
            PipewireInternalMessage::CreateDeviceNode(props, result) => {
                let _ = result.send(manager.borrow_mut().create_node(props));
            }
            PipewireInternalMessage::CreateFilterNode(props, result) => {
                let _ = result.send(manager.borrow_mut().create_filter(props));
            }
            PipewireInternalMessage::CreateDeviceLink(source, destination, sender, result) => {
                let _ = result.send(
                    manager
                        .borrow_mut()
                        .create_link(source, destination, sender),
                );
            }

            PipewireInternalMessage::RemoveDeviceNode(id, result) => {
                let _ = result.send(manager.borrow_mut().remove_node(id));
            }

            PipewireInternalMessage::RemoveDeviceLink(source, destination, result) => {
                let _ = result.send(manager.borrow_mut().remove_link(source, destination));
            }
            PipewireInternalMessage::RemoveFilterNode(ulid, result) => {
                let _ = result.send(manager.borrow_mut().remove_filter(ulid));
            }

            PipewireInternalMessage::DestroyUnmanagedLinks(id, result) => {
                let _ = result.send(manager.borrow_mut().remove_all_unmanaged_links(id));
            }

            PipewireInternalMessage::GetFilterParameters(id, result) => {
                let _ = result.send(manager.borrow_mut().get_filter_values(id));
            }

            PipewireInternalMessage::SetFilterValue(id, key, value, result) => {
                let _ = result.send(manager.borrow_mut().set_filter_value(id, key, value));
            }

            PipewireInternalMessage::SetNodeVolume(id, volume, result) => {
                let _ = result.send(manager.borrow_mut().set_node_volume(id, volume));
            }

            PipewireInternalMessage::SetNodeMute(id, mute, result) => {
                let _ = result.send(manager.borrow_mut().set_node_mute(id, mute));
            }

            PipewireInternalMessage::SetApplicationTarget(id, target, result) => {
                let _ = result.send(manager.borrow_mut().set_application_target(id, target));
            }

            PipewireInternalMessage::SetApplicationVolume(id, volue, result) => {
                let _ = result.send(manager.borrow_mut().set_application_volume(id, volue));
            }
            PipewireInternalMessage::SetApplicationMute(id, state, result) => {
                let _ = result.send(manager.borrow_mut().set_application_muted(id, state));
            }

            PipewireInternalMessage::SetDeviceVolume(id, volume, result) => {
                let _ = result.send(manager.borrow_mut().set_device_volume(id, volume));
            }
            PipewireInternalMessage::SetDeviceMute(id, muted, result) => {
                let _ = result.send(manager.borrow_mut().set_device_muted(id, muted));
            }

            PipewireInternalMessage::SetDefaultDevice(class, node, result) => {
                let _ = result.send(manager.borrow_mut().set_default_device(class, node));
            }

            PipewireInternalMessage::ClearApplicationTarget(id, result) => {
                let _ = result.send(manager.borrow_mut().clear_application_target(id));
            }
        }
    });

    debug!("Pipewire Initialised, starting mainloop");
    start_tx.send(Ok(())).expect("OneShot Channel is broken!");
    mainloop.run();

    let _ = callback_tx.send(PipewireReceiver::Exited);

    info!("[PIPEWIRE] Main Loop Terminated");
}
