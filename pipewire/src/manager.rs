use crate::registry::PipewireRegistry;
use crate::store::{FilterStore, LinkStore, LinkStoreMap, NodeStore, PortLocation, Store};
use crate::{
    registry, FilterHandler, FilterProperties, FilterValue, LinkType, NodeProperties,
    PipewireInternalMessage, PipewireReceiver,
};
use crate::{MediaClass, PWReceiver, PipewireMessage};
use anyhow::Result;
use anyhow::{anyhow, bail};
use log::{debug, error, info};
use pipewire::core::Core;
use pipewire::filter::{Filter, FilterFlags, FilterState, PortFlags};
use pipewire::keys::{APP_ICON_NAME, APP_ID, APP_NAME, AUDIO_CHANNEL, AUDIO_CHANNELS, DEVICE_ICON_NAME, FACTORY_NAME, FORMAT_DSP, LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, MEDIA_CATEGORY, MEDIA_CLASS, MEDIA_ICON_NAME, MEDIA_ROLE, MEDIA_TYPE, NODE_DESCRIPTION, NODE_DRIVER, NODE_FORCE_QUANTUM, NODE_FORCE_RATE, NODE_ID, NODE_LATENCY, NODE_MAX_LATENCY, NODE_NAME, NODE_NICK, NODE_PASSIVE, NODE_VIRTUAL, OBJECT_LINGER, PORT_MONITOR, PORT_NAME};
use pipewire::link::{Link, LinkListener, LinkState};
use pipewire::node::NodeChangeMask;
use pipewire::properties::properties;
use pipewire::proxy::ProxyT;
use pipewire::registry::Registry;
use pipewire::spa::pod::builder::Builder;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{object, Object, Pod, Property, PropertyFlags, Value, ValueArray};
use pipewire::spa::sys::{spa_process_latency_build, spa_process_latency_info, SPA_FORMAT_AUDIO_position, SPA_PARAM_PORT_CONFIG_format, SPA_PARAM_PortConfig, SPA_PARAM_Props, SPA_PROP_channelVolumes, SPA_TYPE_OBJECT_ParamProcessLatency, SPA_TYPE_OBJECT_Props, SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR, SPA_PARAM_INFO_SERIAL};
use pipewire::spa::utils::Direction;

use enum_map::{enum_map, EnumMap};
use oneshot::Sender;
use parking_lot::RwLock;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::serialize::{PodSerialize, PodSerializer};
use pipewire::spa::utils;
use pipewire::sys::pw_impl_link_destroy;
use pipewire::{context, main_loop};
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc;
use strum::IntoEnumIterator;
use ulid::Ulid;

static SAMPLE_RATE: i32 = 48000;

pub(crate) struct FilterData {
    pub callback: Box<dyn FilterHandler>,
}

struct PipewireManager {
    core: Core,
    registry: PipewireRegistry,

    store: Rc<RefCell<Store>>,
    callback_tx: mpsc::Sender<PipewireReceiver>,
}

impl PipewireManager {
    pub fn new(
        core: Core,
        registry: Registry,
        callback_tx: mpsc::Sender<PipewireReceiver>,
    ) -> Self {
        let store = Rc::new(RefCell::new(Store::new(callback_tx.clone())));
        let registry = PipewireRegistry::new(registry, store.clone());

        Self {
            core,
            registry,
            store,
            callback_tx,
        }
    }

    pub fn create_node(&mut self, properties: NodeProperties) -> Result<()> {
        let node_properties = &mut properties! {
            *FACTORY_NAME => "support.null-audio-sink",
            *NODE_NAME => properties.node_name.clone(),
            *NODE_NICK => properties.node_nick,
            *NODE_DESCRIPTION => properties.node_description,

            *NODE_VIRTUAL => "true",
            *PORT_MONITOR => "false",

            *APP_ICON_NAME => &*properties.app_id,
            *MEDIA_ICON_NAME => &*properties.app_id,
            *DEVICE_ICON_NAME => &*properties.app_id,

            //*APP_NAME => properties.app_name,
            *OBJECT_LINGER => match properties.linger {
                true => "true",
                false => "false"
            },
            *MEDIA_CLASS => match properties.class {
                MediaClass::Source => "Audio/Source/Virtual",
                MediaClass::Duplex => "Audio/Duplex",
                MediaClass::Sink => "Audio/Sink",
            },

            *AUDIO_CHANNELS => "2",
            *NODE_LATENCY => format!("{}/{}", properties.buffer, SAMPLE_RATE),
            *NODE_MAX_LATENCY => format!("{}/{}", properties.buffer, SAMPLE_RATE),

            // Force the QUANTUM and the RATE to ensure that we're not internally adjusted when
            // latency occurs following a link
            *NODE_FORCE_QUANTUM => properties.buffer.to_string(),
            *NODE_FORCE_RATE => SAMPLE_RATE.to_string(),

            // We don't want to set a driver here. If creating a large number of nodes each of them
            // will pick a different device while finding a clock source, resulting in the nodes
            // being spread all over the place. When the node tree starts getting linked together
            // pipewire needs to pull all the nodes / audio_filters / devices into a single clock source
            // which can cause some pretty aggressive behaviours (I've seen it infinite loop as
            // various nodes fight for clock control).
            //
            // Setting this to false means that the devices will fall under the 'Dummy' node until
            // a physical device is attached, at which point it'll move everything together under
            // that single clock.
            *NODE_DRIVER => "false",

            // https://gitlab.freedesktop.org/pipewire/pipewire/-/wikis/Virtual-Devices
            "audio.position" => "FL,FR",

            // In the case of this app, we're handling the volumes ourselves via audio_filters, so
            // we're going to simply ignore what pipewire says the volume is and monitor at 100%.
            // This should prevent weirdness if the volumes are directly adjusted.
            "monitor.channel-volumes" => "false",

            // Keep the monitor as close to 'real-time' as possible
            "monitor.passthrough" => "true",
        };

        debug!(
            "[{}] Attempting to Create Device '{}'",
            properties.node_id, properties.node_name
        );

        // Properties built, create the node.
        let proxy = self
            .core
            .create_object::<pipewire::node::Node>("adapter", node_properties)
            .map_err(|e| anyhow!("Unable to Create Node {}", e))?;

        // Set the Initial volume
        let volume = (properties.initial_volume as f32 / 100.0).powi(3);
        let pod = Value::Object(object! {
            utils::SpaTypes::ObjectParamProps,
            ParamType::Props,
            Property::new(SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(vec![volume, volume]))),
        });

        let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &pod)?;
        let bytes = cursor.into_inner();
        if let Some(bytes) = Pod::from_bytes(&bytes) {
            proxy.set_param(ParamType::Props, 0, bytes);
        }

        debug!("[{}] Registering Proxy Listener", properties.node_id);
        let proxy_id = properties.node_id;
        let proxy_store = self.store.clone();
        let proxy_listener = proxy
            .upcast_ref()
            .add_listener_local()
            .bound(move |id| {
                debug!("[{}] Pipewire NodeID assigned: {}", proxy_id, id);
                proxy_store
                    .borrow_mut()
                    .managed_node_set_pw_id(proxy_id, id);
            })
            .removed(|| {
                debug!("Removed..");
            })
            .register();

        debug!("[{}] Registering Node Listener", properties.node_id);
        let listener_id = properties.node_id;
        let listener_info_store = self.store.clone();
        let listener_param_store = self.store.clone();
        let listener = proxy
            .add_listener_local()
            .info(move |info| {
                // Check whether this is a PORT related message
                if info.change_mask().contains(NodeChangeMask::INPUT_PORTS)
                    || info.change_mask().contains(NodeChangeMask::OUTPUT_PORTS)
                {
                    // Now check whether our port count matches what's expected
                    if info.n_input_ports() == 2 && info.n_output_ports() == 2 {
                        debug!(
                            "[{}] Ports have appeared, requesting configuration",
                            listener_id
                        );
                        listener_info_store
                            .borrow()
                            .managed_node_request_ports(listener_id);
                    }
                }
            })
            .param(move |_seq, _type, _index, _next, param| {
                if let Some(pod) = param {
                    let pod = PodDeserializer::deserialize_any_from(pod.as_bytes()).map(|(_, v)| v);
                    if let Ok(Value::Object(object)) = pod {
                        if object.id == SPA_PARAM_PortConfig {
                            debug!("[{}] Port configuration Received", listener_id);
                            let prop = object
                                .properties
                                .iter()
                                .find(|p| p.key == SPA_PARAM_PORT_CONFIG_format);

                            // Format is optional
                            if let Some(prop) = prop {
                                if let Value::Object(object) = &prop.value {
                                    // Value is of type SPA_TYPE_OBJECT_Format
                                    let prop = object
                                        .properties
                                        .iter()
                                        .find(|p| p.key == SPA_FORMAT_AUDIO_position);

                                    if let Some(prop) = prop {
                                        // Fucking hell, I hate how deep this is getting
                                        if let Value::ValueArray(ValueArray::Id(array)) =
                                            &prop.value
                                        {
                                            let mut store = listener_param_store.borrow_mut();
                                            for (index, value) in array.iter().enumerate() {
                                                let index = index as u32;
                                                if value.0 == SPA_AUDIO_CHANNEL_FL {
                                                    store.managed_node_add_port(
                                                        listener_id,
                                                        PortLocation::LEFT,
                                                        index,
                                                    );
                                                }
                                                if value.0 == SPA_AUDIO_CHANNEL_FR {
                                                    store.managed_node_add_port(
                                                        listener_id,
                                                        PortLocation::RIGHT,
                                                        index,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if object.id == SPA_PARAM_Props {
                            let prop = object
                                .properties
                                .iter()
                                .find(|p| p.key == SPA_PROP_channelVolumes);

                            // Get the Left / Right value
                            if let Some(prop) = prop {
                                if let Value::ValueArray(ValueArray::Float(value)) = &prop.value {
                                    // OK, so KDE and pwvucontrol use the highest value for their reference
                                    let max = value
                                        .iter()
                                        .copied()
                                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                                        .unwrap();

                                    let volume = (max.cbrt() * 100.0).round() as u8;
                                    listener_param_store.borrow_mut().on_volume_change(listener_id, volume);
                                }
                            }
                        } else {
                            error!("Parameter Parse Error, Message was not of expected type");
                            debug!("Object Id: {}", object.id);
                            for property in object.properties {
                                debug!("Key: {}, Value: {:?}", property.key, property.value);
                            }
                        }
                    } else {
                        error!("Unexpected Value Type");
                    }
                }
            })
            .register();
        proxy.subscribe_params(&[ParamType::Props]);

        let store = NodeStore {
            pw_id: None,
            object_serial: None,
            id: properties.node_id,
            props: node_properties.clone(),
            proxy,
            listener,
            proxy_listener,

            port_map: Default::default(),
            ports_ready: false,

            ready_sender: Some(properties.ready_sender),
        };

        self.store.borrow_mut().managed_node_add(store);

        Ok(())
    }

    pub fn remove_node(&mut self, id: Ulid) -> Result<()> {
        self.store.borrow_mut().managed_node_remove(id);
        Ok(())
    }

    pub fn create_filter(&mut self, props: FilterProperties) -> Result<()> {
        // For now, we assume a mono implementation... We should separately support both varying
        // input and output counts and have upstream handle it
        let properties = properties!(
            *APP_ID => &*props.app_id,
            *NODE_NAME => &*props.filter_name,
            *NODE_NICK => &*props.filter_nick,
            *NODE_DESCRIPTION => &*props.filter_description,

            *MEDIA_TYPE => "Audio",
            *MEDIA_CATEGORY => "Filter",
            *MEDIA_ROLE => "DSP",

            *OBJECT_LINGER => "false",
        );

        debug!(
            "[{}] Attempting to Create Filter '{}'",
            props.filter_id, props.filter_name
        );
        let filter = Filter::new(&self.core, &props.filter_name, properties)
            .map_err(|e| anyhow!("Unable to Create Filter: {}", e))?;
        let mut params = [];

        // Create port storage
        let input_ports = Rc::new(RefCell::new(vec![]));
        let output_ports = Rc::new(RefCell::new(vec![]));

        let mut input_port_map = EnumMap::default();
        let mut output_port_map = EnumMap::default();

        if props.class == MediaClass::Source || props.class == MediaClass::Duplex {
            debug!("[{}] Registering Input Ports", props.filter_id);
            for (index, port) in PortLocation::iter().enumerate() {
                input_ports.borrow_mut().push(
                    filter
                        .add_port(
                            Direction::Input,
                            PortFlags::MAP_BUFFERS,
                            properties! {
                                *FORMAT_DSP => "32 bit float mono audio",
                                *PORT_NAME => format!("input_{}", port),
                                *AUDIO_CHANNEL => format!("{}", port)
                            },
                            &mut params,
                        )
                        .map_err(|e| anyhow!("Filter Input Creation Failed: {}", e))?,
                );
                input_port_map[port] = index as u32;
            }
        }

        #[allow(clippy::collapsible_if)]
        //if !props.receive_only {
        if props.class == MediaClass::Sink || props.class == MediaClass::Duplex {
            debug!("[{}] Registering Output Ports", props.filter_id);

            for (index, port) in PortLocation::iter().enumerate() {
                output_ports.borrow_mut().push(
                    filter
                        .add_port(
                            Direction::Output,
                            PortFlags::MAP_BUFFERS,
                            properties! {
                                    *FORMAT_DSP => "32 bit float mono audio",
                                    *PORT_NAME => format!("output_{}", port),
                                    *AUDIO_CHANNEL => format!("{}", port)
                                },
                            &mut params,
                        )
                        .map_err(|e| anyhow!("Filter Input Creation Failed: {:?}", e))?,
                );
                output_port_map[port] = index as u32;
            }
        }
        //}

        // Use a RWLock provided by parking-lot here, so we can safely grab the filter to change
        // its settings on-the-fly
        let data = Rc::new(RwLock::new(FilterData {
            callback: props.callback,
        }));
        let data_inner = data.clone();

        debug!("[{}] Registering Filter Listener", props.filter_id);
        let listener_input_ports = input_ports.clone();
        let listener_output_ports = output_ports.clone();
        let listener_state_store = self.store.clone();
        let listener_id = props.filter_id;
        let listener = filter
            .add_local_listener_with_user_data(data_inner)
            .state_changed(move |filter, _data, old, _new| {
                if old == FilterState::Connecting {
                    debug!("[{}] Filter Connected", listener_id);
                    listener_state_store
                        .borrow_mut()
                        .managed_filter_set_pw_id(listener_id, filter.node_id());
                }
            })
            .process(move |filter, data, position| {
                let samples = position.clock.duration as u32;
                //debug!("Rate: {:?}", position.clock.rate.denom);

                let mut input_list = vec![];
                let mut output_list = vec![];
                for input in listener_input_ports.borrow().iter() {
                    let in_buffer = filter.get_dsp_buffer::<f32>(input, samples);
                    input_list.push(in_buffer.unwrap());
                }

                for output in listener_output_ports.borrow().iter() {
                    let out_buffer = filter.get_dsp_buffer::<f32>(output, samples);
                    output_list.push(out_buffer.unwrap());
                }

                data.write()
                    .callback
                    .process_samples(input_list, output_list);
            })
            .register()
            .map_err(|e| anyhow!("Unable to Register Filter: {:?}", e))?;

        let mut buffer = vec![];
        let builder = Builder::new(&mut buffer);

        let latency = spa_process_latency_info {
            quantum: 0.,
            rate: 0,
            ns: 1,
        };
        let pod = unsafe {
            Pod::from_raw(spa_process_latency_build(
                builder.as_raw_ptr(),
                SPA_TYPE_OBJECT_ParamProcessLatency,
                &latency,
            ))
        };
        let mut params = [pod];

        debug!("[{}] Connecting Filter", props.filter_id);
        filter
            .connect(FilterFlags::RT_PROCESS, &mut params)
            .map_err(|e| anyhow!("Unable to Connect Filter: {}", e))?;

        let store = FilterStore {
            pw_id: None,
            data,

            id: props.filter_id,
            _listener: listener,
            _filter: filter,

            port_map: enum_map! {
                registry::Direction::In => input_port_map,
                registry::Direction::Out=> output_port_map,
            },

            input_ports,
            output_ports,

            ready_sender: Some(props.ready_sender),
        };

        self.store.borrow_mut().managed_filter_add(store);

        Ok(())
    }

    pub fn remove_filter(&mut self, id: Ulid) -> Result<()> {
        self.store.borrow_mut().managed_filter_remove(id);
        Ok(())
    }

    pub fn set_filter_value(&mut self, id: Ulid, key: u32, value: FilterValue) -> Result<()> {
        // We need to grab the filter from the store, and pass the value set..
        self.store
            .borrow_mut()
            .managed_filter_set_parameter(id, key, value);
        Ok(())
    }

    pub fn create_link(
        &mut self,
        source: LinkType,
        dest: LinkType,
        sender: Option<Sender<()>>,
    ) -> Result<()> {
        let parent_id = Ulid::new();
        let mut port_map: EnumMap<PortLocation, Option<LinkStoreMap>> = Default::default();

        // Rewrite, lets go!
        for port in PortLocation::iter() {
            // Firstly, create an id for this list
            let link_id = Ulid::new();

            // Next, obtain the source and destination port indexes
            let (src_id, src_index) = self.get_port(source, registry::Direction::Out, port)?;
            let (tgt_id, tgt_index) = self.get_port(dest, registry::Direction::In, port)?;

            // Now we simply create the link
            let (link, lis) =
                self.create_port_link(link_id, parent_id, src_id, src_index, tgt_id, tgt_index)?;

            // Create the LinkStore Mapping for this link
            let store = LinkStoreMap {
                pw_id: None,
                internal_id: link_id,
                link,
                _listener: lis,
                source_port_id: src_index,
                destination_port_id: tgt_index,
            };

            port_map[port] = Some(store);
        }

        // Ok, we're done here, create the main store object
        let group = LinkStore {
            source,
            destination: dest,
            links: port_map,
            ready_sender: sender,
        };

        self.store.borrow_mut().managed_link_add(parent_id, group);
        Ok(())
    }

    pub fn remove_link(&mut self, source: LinkType, destination: LinkType) -> Result<()> {
        self.store
            .borrow_mut()
            .managed_link_remove(source, destination);
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

    fn get_port(
        &mut self,
        link: LinkType,
        direction: registry::Direction,
        location: PortLocation,
    ) -> Result<(u32, u32)> {
        // Ok, simple enough, pull out the relevant type, and get the port at location
        let mut store = self.store.borrow_mut();
        match link {
            LinkType::Node(id) => {
                let node = store.managed_node_get(id).unwrap();

                let id = node.pw_id.unwrap();
                let port = node.port_map[location].unwrap();

                Ok((id, port))
            }
            LinkType::Filter(id) => {
                let filter = store.managed_filter_get(id).unwrap();

                let id = filter.pw_id.unwrap();
                let port = filter.port_map[direction][location];

                Ok((id, port))
            }
            LinkType::UnmanagedNode(id) => {
                let node = store
                    .unmanaged_device_node_get(id)
                    .ok_or_else(|| anyhow!("Unmanaged Device Node not Found"))?;

                let ports = &node.ports[direction];

                // Check whether this is a mono device
                if ports.iter().count() == 1 {
                    if let Some(index) = ports.keys().next() {
                        return Ok((id, *index));
                    }
                }

                // Iterate over the ports, try and find the location
                for (index, port) in ports.iter() {
                    if let Ok(port_location) = PortLocation::from_str(&port.channel) {
                        if port_location == location {
                            return Ok((id, *index));
                        }
                    }
                }

                // If we get here, we didn't find anything, this shouldn't happen!
                bail!("Requested Unmanaged Node is Neither Stereo or Mono");
            }
        }
    }

    fn create_port_link(
        &self,
        id: Ulid,
        parent_id: Ulid,
        src_node: u32,
        src_port: u32,
        dest_node: u32,
        dest_port: u32,
    ) -> Result<(Link, LinkListener)> {
        let listener_info_store = self.store.clone();
        let link = self
            .core
            .create_object::<Link>(
                "link-factory",
                &properties! {
                    *LINK_OUTPUT_NODE => src_node.to_string(),
                    *LINK_OUTPUT_PORT => src_port.to_string(),
                    *LINK_INPUT_NODE => dest_node.to_string(),
                    *LINK_INPUT_PORT => dest_port.to_string(),
                    *OBJECT_LINGER => "false",

                    // No passivity here. While our links may, in some cases, be attached to
                    // physical sources / sinks, in other cases they're attached to audio_filters which
                    // don't have the opportunity to go idle, and implying as such can create a
                    // disconnect between internal and external behaviours.
                    //
                    // TODO: send a parameter indicating the node types
                    *NODE_PASSIVE => "false",
                },
            )
            .map_err(|e| anyhow!("Failed to create link: {}", e))?;

        let link_listener = link
            .add_listener_local()
            .info(move |link| {
                if let LinkState::Active = link.state() {
                    // We're alive, let the store know
                    listener_info_store
                        .borrow_mut()
                        .managed_link_ready(parent_id, id, link.id());
                }
            })
            .register();

        Ok((link, link_listener))
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
            store.unmanaged_node_set_meta(app_id, String::from("target.node"), Some(String::from("Spa:Id")), Some(pw_id.to_string()));
        }
        if let Some(serial) = object_serial {
            store.unmanaged_node_set_meta(app_id, String::from("target.object"), Some(String::from("Spa:Id")), Some(serial.to_string()));
        }

        Ok(())
    }

    fn set_node_volume(&mut self, id: Ulid, volume: u8) -> Result<()> {
        self.store.borrow_mut().set_volume(id, volume)
    }
}

pub fn run_pw_main_loop(
    pw_rx: PWReceiver,
    start_tx: oneshot::Sender<anyhow::Result<()>>,
    callback_tx: mpsc::Sender<PipewireReceiver>,
) {
    debug!("Initialising Pipewire..");

    let Ok(mainloop) = main_loop::MainLoop::new(None) else {
        start_tx
            .send(Err(anyhow!("Unable to create MainLoop")))
            .expect("OneShot Channel is broken!");
        return;
    };
    let Ok(context) = context::Context::new(&mainloop) else {
        start_tx
            .send(Err(anyhow!("Unable to create Context")))
            .expect("OneShot Channel is broken!");
        return;
    };

    // Wrap the mainloop so we shuffle it around
    let mainloop = Rc::new(mainloop);

    // Now we create a core, and flag it as a manager
    let Ok(core) = context.connect(Some(properties!(
        *MEDIA_CATEGORY => "Manager",
    ))) else {
        start_tx
            .send(Err(anyhow!("Unable to Fetch Core from Context")))
            .expect("OneShot Channel is broken!");
        return;
    };

    let Ok(registry) = core.get_registry() else {
        start_tx
            .send(Err(anyhow!("Unable to Fetch Registry from Core")))
            .expect("OneShot Channel is broken!");
        return;
    };

    let manager = Rc::new(RefCell::new(PipewireManager::new(
        core,
        registry,
        callback_tx,
    )));

    let receiver_clone = mainloop.clone();
    let _receiver = pw_rx.attach(mainloop.loop_(), {
        move |message| match message {
            PipewireInternalMessage::Quit(result) => {
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

            PipewireInternalMessage::SetFilterValue(id, key, value, result) => {
                let _ = result.send(manager.borrow_mut().set_filter_value(id, key, value));
            }

            PipewireInternalMessage::SetNodeVolume(id, volume, result) => {
                let _ = result.send(manager.borrow_mut().set_node_volume(id, volume));
            }

            PipewireInternalMessage::SetApplicationTarget(id, target, result) => {
                let _ = result.send(manager.borrow_mut().set_application_target(id, target));
            }
        }
    });

    debug!("Pipewire Initialised, starting mainloop");
    start_tx.send(Ok(())).expect("OneShot Channel is broken!");
    mainloop.run();

    info!("[PIPEWIRE] Main Loop Terminated");
}
