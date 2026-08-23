//! Lifecycle of the filters we create (e.g. volume/mute processing nodes
//! implemented as pipewire filters rather than plain nodes), and access to
//! their runtime parameters.

use super::{ManagedFilter, PortLocation, Store};
use crate::{Direction, FilterProperties, FilterProperty, FilterValue, LinkType, MediaClass};
use anyhow::{Result, anyhow};
use enum_map::{EnumMap, enum_map};
use log::debug;
use parking_lot::RwLock;
use pipewire::core::CoreRc;
use pipewire::filter::{FilterFlags, FilterRc, FilterState, PortFlags};
use pipewire::keys::{
    APP_ID, AUDIO_CHANNEL, FORMAT_DSP, MEDIA_CATEGORY, MEDIA_ROLE, MEDIA_TYPE, NODE_ALWAYS_PROCESS,
    NODE_DESCRIPTION, NODE_GROUP, NODE_NAME, NODE_NICK, OBJECT_LINGER, PORT_NAME,
};
use pipewire::properties::properties;
use pipewire::spa::pod::Pod;
use pipewire::spa::pod::builder::Builder;
use pipewire::spa::sys::{
    SPA_TYPE_OBJECT_ParamProcessLatency, spa_process_latency_build, spa_process_latency_info,
};
use pipewire::spa::utils;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use strum::IntoEnumIterator;
use ulid::Ulid;

use crate::manager::FilterData;

impl Store {
    pub fn create_filter(
        &mut self,
        core: &CoreRc,
        props: FilterProperties,
        store: Weak<RefCell<Store>>,
    ) -> Result<()> {
        // For now, we assume a mono implementation... We should separately support both varying
        // input and output counts and have upstream handle it
        let properties = properties!(
            *APP_ID => &*props.app_id,
            *NODE_NAME => &*props.filter_name,
            *NODE_NICK => &*props.filter_nick,
            *NODE_DESCRIPTION => &*props.filter_description,

            // READ NOTE IN state_changed BEFORE CHANGING THIS VALUE!
            *NODE_ALWAYS_PROCESS => "true",

            *NODE_GROUP => "pipeweaver-nodes",

            *MEDIA_TYPE => "Audio",
            *MEDIA_CATEGORY => "Filter",
            *MEDIA_ROLE => "DSP",

            *OBJECT_LINGER => "false",
        );

        debug!(
            "[{}] Attempting to Create Filter '{}'",
            props.filter_id, props.filter_name
        );
        let filter = FilterRc::new(core.clone(), &props.filter_name, properties)
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
                            utils::Direction::Input,
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

        if props.class == MediaClass::Sink || props.class == MediaClass::Duplex {
            debug!("[{}] Registering Output Ports", props.filter_id);

            for (index, port) in PortLocation::iter().enumerate() {
                output_ports.borrow_mut().push(
                    filter
                        .add_port(
                            utils::Direction::Output,
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

        // Use a RWLock provided by parking-lot here, so we can safely grab the filter to change
        // its settings on-the-fly
        let data = Rc::new(RwLock::new(FilterData {
            callback: props.callback,
        }));
        let data_inner = data.clone();

        debug!("[{}] Registering Filter Listener", props.filter_id);
        let listener_input_ports = input_ports.clone();
        let listener_output_ports = output_ports.clone();
        let listener_state_store = store.clone();
        let listener_id = props.filter_id;
        let listener = filter
            .add_local_listener_with_user_data(data_inner)
            .state_changed(move |filter, _data, _old, new| {
                // Note, this ONLY works because NODE_ALWAYS_PROCESS is true. There's no way via
                // the filter API to know when the ports have appeared, meaning it would have to
                // be tracked in the global registry handler, however, because we're always process
                // we enter a streaming state once all our ports arrive, meaning that we don't have
                // to track them directly.
                //
                // TODO: We should probably track the ports in the global registry handler anyway.
                if new == FilterState::Streaming {
                    debug!("[{}] Filter Connected and Ready", listener_id);
                    if let Some(listener_state_store) = listener_state_store.upgrade() {
                        let mut store = listener_state_store.borrow_mut();
                        store.managed_filter_set_pw_id(listener_id, filter.node_id());
                        store.managed_filter_send(listener_id);
                    }
                }
            })
            .process(move |filter, data, position| {
                let samples = position.clock.duration as u32;

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

                // Check for inputs, output only filters don't need this
                if !input_list.is_empty() {
                    // Iterate over all the output lists
                    for (i, out_buf) in output_list.iter_mut().enumerate() {
                        // Fetch the matching input, if it's empty and the output ISN'T..
                        if !out_buf.is_empty() && input_list.get(i).is_none_or(|b| b.is_empty()) {
                            // Clear the output buffer
                            out_buf.fill(0.0);
                        }
                    }
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

        let filter = ManagedFilter {
            pw_id: None,
            data,

            id: props.filter_id,
            _listener: listener,
            _filter: filter,

            port_map: enum_map! {
                Direction::In => input_port_map,
                Direction::Out=> output_port_map,
            },

            _input_ports: input_ports,
            _output_ports: output_ports,

            ready_sender: Some(props.ready_sender),
        };

        self.managed_filter_add(filter);

        Ok(())
    }

    pub fn managed_filter_add(&mut self, filter: ManagedFilter) {
        debug!("[{}] Filter Added to Store", filter.id);
        self.managed_filters.insert(filter.id, filter);
    }

    pub fn managed_filter_send(&mut self, id: Ulid) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        if let Some(sender) = filter.take_ready_sender() {
            let _ = sender.send(());
        }
    }

    pub fn managed_filter_get(&self, id: Ulid) -> Option<&ManagedFilter> {
        self.managed_filters.get(&id)
    }

    pub fn managed_filter_remove(&mut self, filter: Ulid) {
        self.managed_link_remove_for_type(LinkType::Filter(filter));
        self.managed_filters.remove(&filter);
    }

    pub fn managed_filter_set_pw_id(&mut self, id: Ulid, pw_id: u32) {
        let filter = self.managed_filters.get_mut(&id).expect("Broke");
        filter.pw_id = Some(pw_id);
    }

    pub fn managed_filter_set_parameter(
        &mut self,
        id: Ulid,
        key: u32,
        value: FilterValue,
    ) -> Result<String> {
        // Find the filter
        let filter = self
            .managed_filters
            .get_mut(&id)
            .ok_or(anyhow!("Filter Not Found"))?;

        // Set the Property
        filter.data.write().callback.set_property(key, value)
    }

    pub fn managed_filter_get_parameters(&self, id: Ulid) -> Result<Vec<FilterProperty>> {
        // Find the filter
        let filter = self
            .managed_filters
            .get(&id)
            .ok_or(anyhow!("Filter Missing"))?;

        // Send the Properties
        Ok(filter.data.read().callback.get_properties())
    }
}
