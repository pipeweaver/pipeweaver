use crate::registry::PipewireRegistry;
use crate::store::{FilterStore, LinkStore, NodeStore, Store};
use crate::{registry, FilterHandler, FilterProperties, FilterValue, LinkType, NodeProperties, PipecastNode};
use crate::{MediaClass, PWReceiver, PipewireMessage};
use anyhow::anyhow;
use log::{debug, error};
use pipewire::core::Core;
use pipewire::filter::{Filter, FilterFlags, FilterState, PortFlags};
use pipewire::keys::{APP_ICON_NAME, APP_ID, APP_NAME, AUDIO_CHANNEL, AUDIO_CHANNELS, CLIENT_ID, CLIENT_NAME, DEVICE_DESCRIPTION, DEVICE_ICON_NAME, DEVICE_ID, DEVICE_NAME, DEVICE_NICK, FACTORY_NAME, FORMAT_DSP, LINK_INPUT_NODE, LINK_INPUT_PORT, LINK_OUTPUT_NODE, LINK_OUTPUT_PORT, MEDIA_CATEGORY, MEDIA_CLASS, MEDIA_ICON_NAME, MEDIA_ROLE, MEDIA_TYPE, NODE_ALWAYS_PROCESS, NODE_DESCRIPTION, NODE_DRIVER, NODE_FORCE_QUANTUM, NODE_FORCE_RATE, NODE_ID, NODE_LATENCY, NODE_MAX_LATENCY, NODE_NAME, NODE_NICK, NODE_PASSIVE, NODE_VIRTUAL, OBJECT_LINGER, PORT_DIRECTION, PORT_MONITOR, PORT_NAME};
use pipewire::link::Link;
use pipewire::node::NodeChangeMask;
use pipewire::properties::properties;
use pipewire::proxy::ProxyT;
use pipewire::registry::Registry;
use pipewire::spa::pod::builder::Builder;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{Pod, Value, ValueArray};
use pipewire::spa::sys::{spa_process_latency_build, spa_process_latency_info, SPA_FORMAT_AUDIO_position, SPA_PARAM_PORT_CONFIG_format, SPA_PARAM_PortConfig, SPA_TYPE_OBJECT_ParamProcessLatency, SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR, SPA_KEY_AUDIO_POSITION};
use pipewire::spa::utils::Direction;

use pipewire::{context, main_loop};
use std::cell::RefCell;

use parking_lot::RwLock;
use std::rc::Rc;
use ulid::Ulid;

pub(crate) struct FilterData {
    pub callback: Box<dyn FilterHandler>,
}

struct PipewireManager {
    core: Core,
    registry: PipewireRegistry,

    store: Rc<RefCell<Store>>,
}

impl PipewireManager {
    pub fn new(core: Core, registry: Registry) -> Self {
        let store = Rc::new(RefCell::new(Store::new()));
        let registry = PipewireRegistry::new(registry, store.clone());

        Self {
            core,
            registry,
            store,
        }
    }

    pub fn create_node(&mut self, properties: NodeProperties) {
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

            *APP_NAME => properties.app_name,
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
            *NODE_LATENCY => "128/48000",
            *NODE_MAX_LATENCY => "128/48000",


            // Force the QUANTUM and the RATE to ensure that we're not internally adjusted when
            // latency occurs following a link
            *NODE_FORCE_QUANTUM => "128",
            *NODE_FORCE_RATE => "48000",

            // We don't want to set a driver here. If creating a large number of nodes each of them
            // will pick a different device while finding a clock source, resulting in the nodes
            // being spread all over the place. When the node tree starts getting linked together
            // pipewire needs to pull all the nodes / filters / devices into a single clock source
            // which can cause some pretty aggressive behaviours (I've seen it infinite loop as
            // various nodes fight for clock control).
            //
            // Setting this to false means that the devices will fall under the 'Dummy' node until
            // a physical device is attached, at which point it'll move everything together under
            // that single clock.
            *NODE_DRIVER => "false",

            // https://gitlab.freedesktop.org/pipewire/pipewire/-/wikis/Virtual-Devices
            "audio.position" => "FL,FR",

            // In the case of PipeCast, we're handling the volumes ourselves via filters, so we're
            // going to simply ignore what pipewire says the volume is and monitor at 100%. This
            // should prevent weirdness and unexpected results if the volumes are directly adjusted.
            "monitor.channel-volumes" => "false",

            // Keep the monitor as close to 'real-time' as possible
            "monitor.passthrough" => "false",
        };


        debug!(
            "[{}] Attempting to Create Device '{}'",
            properties.node_id, properties.node_name
        );

        // Properties built, create the node.
        let proxy = self
            .core
            .create_object::<pipewire::node::Node>("adapter", node_properties)
            .expect("Unable to Create Object");

        debug!("[{}] Registering Proxy Listener", properties.node_id);
        let proxy_id = properties.node_id;
        let proxy_store = self.store.clone();
        let proxy_listener = proxy
            .upcast_ref()
            .add_listener_local()
            .bound(move |id| {
                debug!("[{}] Pipewire NodeID assigned: {}", proxy_id, id);
                proxy_store.borrow_mut().node_set_pw_id(proxy_id, id);
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
                // Check whether this is a PORT related message..
                if info.change_mask().contains(NodeChangeMask::INPUT_PORTS)
                    || info.change_mask().contains(NodeChangeMask::OUTPUT_PORTS)
                {
                    // Now check whether our port count matches what's expected..
                    if info.n_input_ports() == 2 && info.n_output_ports() == 2 {
                        debug!(
                            "[{}] Ports have appeared, requesting configuration",
                            listener_id
                        );
                        listener_info_store.borrow().node_request_ports(listener_id);
                    }
                }
            })
            .param(move |_seq, _type, _index, _next, _param| {
                //debug!("Seq: {}, ID: {:?}, Index: {}, Next: {}", _seq, _type, _index, _next);
                if let Some(pod) = _param {
                    let pod = PodDeserializer::deserialize_any_from(pod.as_bytes()).map(|(_, v)| v);

                    if let Ok(value) = pod {
                        if let Value::Object(object) = value {
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
                                            // Fucking hell, Now we need to grab the ValueArray
                                            if let Value::ValueArray(ValueArray::Id(array)) =
                                                &prop.value
                                            {
                                                let mut ports = vec![];

                                                for (index, value) in array.iter().enumerate() {
                                                    if value.0 == SPA_AUDIO_CHANNEL_FL {
                                                        debug!(
                                                            "[{}][{}] Front Left",
                                                            listener_id, index
                                                        );
                                                    }
                                                    if value.0 == SPA_AUDIO_CHANNEL_FR {
                                                        debug!(
                                                            "[{}][{}] Front Right",
                                                            listener_id, index
                                                        );
                                                    }

                                                    ports.push(value.0);
                                                }

                                                // In our case, we need to set both the inputs and outputs to match.
                                                listener_param_store.borrow_mut().node_set_ports(
                                                    listener_id,
                                                    true,
                                                    ports.clone(),
                                                );
                                                listener_param_store.borrow_mut().node_set_ports(
                                                    listener_id,
                                                    false,
                                                    ports,
                                                );

                                                // Now we can flag ourselves as ready
                                                listener_param_store
                                                    .borrow_mut()
                                                    .node_ports_ready(listener_id);

                                                return;
                                            }
                                        }
                                    }
                                }
                            }

                            error!("Parameter Parse Error, Message was not of expected type");
                            debug!("Object Id: {}", object.id);
                            for property in object.properties {
                                debug!("Key: {}, Value: {:?}", property.key, property.value);
                            }
                        } else {
                            error!("Unexpected Value Type");
                        }
                    }
                }
            })
            .register();

        let store = NodeStore {
            pw_id: None,
            id: properties.node_id,
            props: node_properties.clone(),
            proxy,
            listener,
            proxy_listener,

            ports_ready: false,
            input_ports: vec![],
            output_ports: vec![],

            ready_sender: Some(properties.ready_sender),
        };

        self.store.borrow_mut().add_node(store);
    }

    pub fn create_filter(&mut self, props: FilterProperties) {
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
        let filter = Filter::new(&self.core, &props.filter_name, properties).expect("Yup");
        let mut params = [];

        // Create port storage
        let mut input_ports = vec![];
        let mut output_ports = vec![];

        debug!("[{}] Registering Input Ports", props.filter_id);
        for i in ["FL", "FR"] {
            input_ports.push(filter.add_port(
                Direction::Input,
                PortFlags::MAP_BUFFERS,
                properties! {*FORMAT_DSP => "32 bit float mono audio", *PORT_NAME => format!("input_{}", i), *AUDIO_CHANNEL => i},
                &mut params,
            ).expect("Filter Input Creation Failed"));
        }

        debug!("[{}] Registering Output Ports", props.filter_id);
        for i in ["FL", "FR"] {
            output_ports.push(filter.add_port(
                Direction::Output,
                PortFlags::MAP_BUFFERS,
                properties! {*FORMAT_DSP => "32 bit float mono audio", *PORT_NAME => format!("output_{}", i), *AUDIO_CHANNEL => i},
                &mut params,
            ).expect("Filter Output Creation Failed"));
        }

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
                        .filter_set_pw_id(listener_id, filter.node_id());
                }
            })
            .process(move |filter, data, position| {
                let samples = position.clock.duration as u32;
                //debug!("Rate: {:?}", position.clock.rate.denom);

                let mut input_list = vec![];
                let mut output_list = vec![];
                for input in &listener_input_ports {
                    let in_buffer = filter.get_dsp_buffer::<f32>(input, samples);
                    input_list.push(in_buffer.unwrap());
                }

                for output in &listener_output_ports {
                    let out_buffer = filter.get_dsp_buffer::<f32>(output, samples);
                    output_list.push(out_buffer.unwrap());
                }

                data.read().callback.process_samples(input_list, output_list);
            })
            .register()
            .expect("Filter Borked.");

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
            .expect("Unable to Connect");

        let store = FilterStore {
            pw_id: None,
            data,

            id: props.filter_id,
            _filter: filter,
            _listener: listener,
            input_ports,
            output_ports,

            ready_sender: Some(props.ready_sender),
        };

        self.store.borrow_mut().add_filter(store);
    }

    pub fn set_filter_value(&mut self, id: Ulid, key: u32, value: FilterValue) {
        // We need to grab the filter from the store, and pass the value set..
        self.store.borrow_mut().filter_set_parameter(id, key, value);
    }

    pub fn create_link(&mut self, source: LinkType, destination: LinkType) {
        // Locate the Source Node..
        let src = self.get_ports(source);
        let dest = self.get_ports(destination);

        // TODO: Fix this too
        // Still making assumptions about the Ports and their mapping, we should check for the
        // canonical setup.
        for i in 0..2 {
            // Check whether we have a mono source, if so we need to map it to stereo
            let src_port = if i == 1 && src.2 == 1 {
                0
            } else {
                i
            };

            // Do the same for the Destination
            let dest_port = if i == 1 && dest.1 == 1 {
                0
            } else {
                i
            };

            let link = self.create_port_link(src.0, src_port as u32, dest.0, dest_port as u32);
            let store = LinkStore {
                link,

                source,
                src_port_id: i,

                destination,
                dest_port_id: i,
            };
            self.store.borrow_mut().add_link(store);
        }
    }

    fn get_ports(&self, link: LinkType) -> (u32, usize, usize) {
        // TODO: Fix this
        // We should instead be correctly mapping the FL, FR and MONO ports here, rather than
        // just hoping they line up
        match link {
            LinkType::Node(id) => {
                let store = self.store.borrow();
                let node = store.get_node(id).unwrap();

                let id = node.pw_id.unwrap();
                let input_port_count = node.input_ports.len();
                let output_port_count = node.output_ports.len();

                (id, input_port_count, output_port_count)
            }
            LinkType::Filter(id) => {
                let store = self.store.borrow();
                let filter = store.get_filter(id).unwrap();

                let id = filter.pw_id.unwrap();
                let input_port_count = filter.input_ports.len();
                let output_port_count = filter.output_ports.len();

                (id, input_port_count, output_port_count)
            }
            LinkType::UnmanagedNode(id) => {
                let mut store = self.store.borrow_mut();
                let node = store.get_unmanaged_node(id).expect("Invalid NodeID");

                let input_port_count = node.ports[registry::Direction::In].len();
                let output_port_count = node.ports[registry::Direction::Out].len();

                (id, input_port_count, output_port_count)
            }
        }
    }

    fn create_port_link(
        &self,
        src_node: u32,
        src_port: u32,
        dest_node: u32,
        dest_port: u32,
    ) -> Link {
        self.core
            .create_object::<pipewire::link::Link>(
                "link-factory",
                &properties! {
                    *LINK_OUTPUT_NODE => src_node.to_string(),
                    *LINK_OUTPUT_PORT => src_port.to_string(),
                    *LINK_INPUT_NODE => dest_node.to_string(),
                    *LINK_INPUT_PORT => dest_port.to_string(),
                    *OBJECT_LINGER => "false",

                    // No passivity here. While our links may, in some cases, be attached to
                    // physical sources / sinks, in other cases they're attached to filters which
                    // don't have the opportunity to go idle, and implying as such can create a
                    // disconnect between internal and external behaviours.
                    //
                    // TODO: send a parameter indicating the node types
                    *NODE_PASSIVE => "false",
                },
            )
            .expect("Failed to create link")
    }

    fn get_usable_nodes(&self) -> Vec<PipecastNode> {
        self.store.borrow().get_usable_nodes()
    }
}

pub fn run_pw_main_loop(pw_rx: PWReceiver, start_tx: oneshot::Sender<anyhow::Result<()>>) {
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


    let manager = Rc::new(RefCell::new(PipewireManager::new(core, registry)));

    let receiver_clone = mainloop.clone();
    let _receiver = pw_rx.attach(mainloop.loop_(), {
        move |message| match message {
            PipewireMessage::Quit => {
                receiver_clone.quit();
            }
            PipewireMessage::CreateDeviceNode(props) => {
                manager.borrow_mut().create_node(props);
            }
            PipewireMessage::CreateFilterNode(props) => {
                manager.borrow_mut().create_filter(props);
            }
            PipewireMessage::CreateDeviceLink(source, destination) => {
                manager.borrow_mut().create_link(source, destination);
            }
            PipewireMessage::GetUsableNodes(sender) => {
                sender.send(manager.borrow().get_usable_nodes()).expect("Broken Response Sender!");
            }

            PipewireMessage::SetFilterValue(id, key, value) => {
                manager.borrow_mut().set_filter_value(id, key, value)
            }
        }
    });

    debug!("Pipewire Initialised, starting mainloop");
    start_tx.send(Ok(())).expect("OneShot Channel is broken!");
    mainloop.run();
}
