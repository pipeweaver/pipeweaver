use crate::default_device::DefaultDefinition;
use crate::store::{Store, TargetType};
use pipewire::metadata::{Metadata, MetadataListener};
use pipewire::registry::{GlobalObject, Registry};
use pipewire::spa::utils::dict::DictRef;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub fn handle_metadata(
    global: &GlobalObject<&DictRef>,
    registry: Rc<RefCell<Registry>>,
    store: &mut Store,
    listener_store: Weak<RefCell<Store>>,
) {
    if let Some(props) = global.props
        && let Some(name) = props.get("metadata.name")
    {
        if name == "default" {
            let proxy: Option<Metadata> = registry.borrow().bind(global).ok();
            if let Some(metadata) = proxy {
                let listen_store = listener_store.clone();
                let listener = metadata
                    .add_listener_local()
                    .property(move |subject, key, _type, value| {
                        if key == Some("target.object") {
                            let target = value.and_then(|s| s.parse::<u32>().ok());
                            if let Some(listen_store) = listen_store.upgrade() {
                                listen_store.borrow_mut().unmanaged_client_node_set_target(
                                    subject,
                                    TargetType::Serial(target),
                                );
                            }
                        }
                        if key == Some("target.node") {
                            let target = value.and_then(|s| s.parse::<u32>().ok());
                            if let Some(listen_store) = listen_store.upgrade() {
                                listen_store.borrow_mut().unmanaged_client_node_set_target(
                                    subject,
                                    TargetType::Node(target),
                                );
                            }
                        }
                        if key == Some("default.audio.sink")
                            && _type == Some("Spa:String:JSON")
                            && let Some(val) = value
                            && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                            && let Some(name) = json.get("name").and_then(|v| v.as_str())
                            && let Some(listen_store) = listen_store.upgrade()
                        {
                            listen_store
                                .borrow_mut()
                                .set_default_sink(DefaultDefinition::Default(String::from(name)));
                        }

                        if key == Some("default.configured.audio.sink")
                            && _type == Some("Spa:String:JSON")
                            && let Some(val) = value
                            && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                            && let Some(name) = json.get("name").and_then(|v| v.as_str())
                            && let Some(listen_store) = listen_store.upgrade()
                        {
                            listen_store.borrow_mut().set_default_sink(
                                DefaultDefinition::Configured(String::from(name)),
                            );
                        }

                        if key == Some("default.audio.source")
                            && _type == Some("Spa:String:JSON")
                            && let Some(val) = value
                            && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                            && let Some(name) = json.get("name").and_then(|v| v.as_str())
                            && let Some(listen_store) = listen_store.upgrade()
                        {
                            listen_store
                                .borrow_mut()
                                .set_default_source(DefaultDefinition::Default(String::from(name)));
                        }

                        if key == Some("default.configured.audio.source")
                            && _type == Some("Spa:String:JSON")
                            && let Some(val) = value
                            && let Ok(json) = serde_json::from_str::<serde_json::Value>(val)
                            && let Some(name) = json.get("name").and_then(|v| v.as_str())
                            && let Some(listen_store) = listen_store.upgrade()
                        {
                            listen_store.borrow_mut().set_default_source(
                                DefaultDefinition::Configured(String::from(name)),
                            );
                        }
                        0
                    })
                    .register();

                let session = MetadataStore {
                    metadata,
                    _listener: listener,
                };
                store.set_session_proxy(session);
            }
        } else if name == "settings" {
            let proxy: Option<Metadata> = registry.borrow().bind(global).ok();
            if let Some(metadata) = proxy {
                let listen_store = listener_store.clone();
                let listener = metadata
                    .add_listener_local()
                    .property(move |_subject, key, _type, value| {
                        if key == Some("clock.rate") {
                            let clock = value.and_then(|s| s.parse::<u32>().ok());
                            if let Some(listen_store) = listen_store.upgrade() {
                                listen_store.borrow_mut().announce_clock_rate(clock);
                            }
                        }
                        0
                    })
                    .register();

                let settings = MetadataStore {
                    metadata,
                    _listener: listener,
                };
                store.set_settings_proxy(settings);
            }
        }
    }
}

pub(crate) struct MetadataStore {
    pub(crate) metadata: Metadata,
    pub(crate) _listener: MetadataListener,
}
