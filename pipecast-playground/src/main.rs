use anyhow::{Result};
use log::{debug, info, LevelFilter};
use pipecast_pipewire::oneshot::Sender;
use pipecast_pipewire::ulid::Ulid;
use pipecast_pipewire::{oneshot, FilterHandler, FilterProperty, FilterValue, MediaClass, PipewireMessage, PipewireRunner};
use pipecast_pipewire::{FilterProperties, LinkType, NodeProperties};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    // Try and make a Pipewire Manager...
    info!("Starting Pipewire Runner..");
    let manager = PipewireRunner::new()?;

    // Ok, define the IDs of all our channels and filters...
    let system_id = Ulid::new();
    let system_a_id = Ulid::new();
    let system_b_id = Ulid::new();

    let game_id = Ulid::new();
    let game_a_id = Ulid::new();
    let game_b_id = Ulid::new();

    let music_id = Ulid::new();
    let music_a_id = Ulid::new();
    let music_b_id = Ulid::new();

    let chat_id = Ulid::new();
    let chat_a_id = Ulid::new();
    let chat_b_id = Ulid::new();

    // Inputs..
    let stream_id = Ulid::new();

    let mic_id = Ulid::new();
    let mic_a_id = Ulid::new();
    let mic_b_id = Ulid::new();

    // These are 'pass-through' filters
    let headphone_pass_id = Ulid::new();
    let microphone_pass_id = Ulid::new();

    // Outputs
    create_node(&manager, system_id, "System", MediaClass::Sink)?;
    create_filter(&manager, system_a_id, "System A", 0.)?;
    create_filter(&manager, system_b_id, "System B", 1.)?;

    create_node(&manager, game_id, "Game", MediaClass::Sink)?;
    create_filter(&manager, game_a_id, "Game A", 1.)?;
    create_filter(&manager, game_b_id, "Game B", 1.)?;

    create_node(&manager, music_id, "Music", MediaClass::Sink)?;
    create_filter(&manager, music_a_id, "Music A", 1.)?;
    create_filter(&manager, music_b_id, "Music B", 1.)?;

    create_node(&manager, chat_id, "Chat", MediaClass::Sink)?;
    create_filter(&manager, chat_a_id, "Chat A", 1.)?;
    create_filter(&manager, chat_b_id, "Chat B", 1.)?;

    // Inputs
    create_node(&manager, stream_id, "Stream Mix", MediaClass::Source)?;

    create_node(&manager, mic_id, "Chat Mic", MediaClass::Source)?;

    // These are passthroughs which will get wired up to 'Real' devices...
    create_pass_through_filter(&manager, headphone_pass_id, "Headphone")?;

    create_pass_through_filter(&manager, microphone_pass_id, "Microphone")?;
    create_filter(&manager, mic_a_id, "Microphone A", 1.)?;
    create_filter(&manager, mic_b_id, "Microphone B", 1.)?;

    // Ok, lets create some routes, we need to route the node to their mixes
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(system_id),
        LinkType::Filter(system_a_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(system_id),
        LinkType::Filter(system_b_id),
    ))?;

    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(game_id),
        LinkType::Filter(game_a_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(game_id),
        LinkType::Filter(game_b_id),
    ))?;

    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(music_id),
        LinkType::Filter(music_a_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(music_id),
        LinkType::Filter(music_b_id),
    ))?;

    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(chat_id),
        LinkType::Filter(chat_a_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Node(chat_id),
        LinkType::Filter(chat_b_id),
    ))?;

    // The Mic Pass-Through needs to be connected to its mixes
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(microphone_pass_id),
        LinkType::Filter(mic_a_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(microphone_pass_id),
        LinkType::Filter(mic_b_id),
    ))?;

    // Now for the Real stuff, everything (inc mic) on Mix B except system to Stream Mix
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(game_b_id),
        LinkType::Node(stream_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(music_b_id),
        LinkType::Node(stream_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(chat_b_id),
        LinkType::Node(stream_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(mic_b_id),
        LinkType::Node(stream_id),
    ))?;

    // Mic A to Chat Mic
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(mic_a_id),
        LinkType::Node(mic_id),
    ))?;

    // Ok, lets create some routes. All 4 'main' channels MixA will route to headphones
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(system_a_id),
        LinkType::Filter(headphone_pass_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(game_a_id),
        LinkType::Filter(headphone_pass_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(music_a_id),
        LinkType::Filter(headphone_pass_id),
    ))?;
    manager.send_message(PipewireMessage::CreateDeviceLink(
        LinkType::Filter(chat_a_id),
        LinkType::Filter(headphone_pass_id),
    ))?;

    let mut volume_done = false;
    let mut volume = 0.;
    loop {
        // Ok, we're going to spend the first 10 seconds raising the volume of System A..
        if !volume_done {
            volume += 0.1;
            manager.send_message(PipewireMessage::SetFilterValue(system_a_id, 0, FilterValue::Float32(volume)))?;
            debug!("Volume Raised to {}", volume);

            if volume >= 1. {
                volume_done = true;
                continue;
            }

            sleep(Duration::from_millis(1000));
            continue;
        }


        sleep(Duration::from_secs(20));
    }
}

fn create_node(manager: &PipewireRunner, id: Ulid, name: &str, class: MediaClass) -> Result<()> {
    let (tx, rx) = oneshot::channel();
    let props = get_node_properties(id, name, class, tx);

    info!("[{}] Requesting Node Creation from Pipewire", id);
    manager.send_message(PipewireMessage::CreateDeviceNode(props))?;

    info!("[{}] Waiting for Pipewire to report device ready", id);
    rx.recv()?;
    info!("[{}] Device Ready!", id);

    Ok(())
}

fn create_filter(manager: &PipewireRunner, id: Ulid, name: &str, volume: f32) -> Result<()> {
    let (tx, rx) = oneshot::channel();
    let props = get_filter_properties(id, name, volume, tx);

    info!("[{}] Requesting Filter Creation from Pipewire", id);
    manager.send_message(PipewireMessage::CreateFilterNode(props))?;

    info!("[{}] Waiting for Pipewire to report filter ready", id);
    rx.recv()?;
    info!("[{}] Filter Ready!", id);

    Ok(())
}

fn create_pass_through_filter(manager: &PipewireRunner, id: Ulid, name: &str) -> Result<()> {
    let (tx, rx) = oneshot::channel();
    let props = get_pass_through_properties(id, name, tx);

    info!("[{}] Requesting Filter Creation from Pipewire", id);
    manager.send_message(PipewireMessage::CreateFilterNode(props))?;

    info!("[{}] Waiting for Pipewire to report filter ready", id);
    rx.recv()?;
    info!("[{}] Filter Ready!", id);

    Ok(())
}

fn get_node_properties(id: Ulid, name: &str, class: MediaClass, tx: Sender<()>) -> NodeProperties {
    NodeProperties {
        node_id: id,
        node_name: format!("PipeCast{}", name),
        node_nick: format!("PipeCast{}", name),
        node_description: format!("PipeCast {}", name),
        app_id: "com.frostycoolslug".to_string(),
        app_name: "pipecast".to_string(),
        linger: false,
        class,
        ready_sender: tx,
    }
}

struct VolumeFilter {
    volume: f32,
}
impl FilterHandler for VolumeFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![FilterProperty {
            id: 0,
            name: "Volume".into(),
            value: FilterValue::Float32(self.volume)
        }]
    }

    fn get_property(&self, id: u32) -> FilterProperty {
        match id {
            0 => FilterProperty {
                id: 0,
                name: "Volume".into(),
                value: FilterValue::Float32(self.volume)
            },
            _ => panic!("Attempted to lookup non-existent property!")
        }
    }

    fn set_property(&mut self, id: u32, value: FilterValue) {
        match id {
            0 => {
                if let FilterValue::Float32(value) = value {
                    self.volume = value;
                } else {
                    panic!("Attempted to Set Volume as non-float!");
                }
            }
            _ => panic!("Attempted to set non-existent property!")
        }
    }

    fn process_samples(&self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() || outputs[i].is_empty() {
                continue;
            }

            // If we're at max volume, just pass through to the output
            if self.volume == 1. {
                outputs[i].copy_from_slice(input);
                continue;
            }

            // Otherwise, multiply the samples by the volume
            for (index, sample) in input.iter().enumerate() {
                outputs[i][index] = sample * self.volume;
            }
        }
    }
}

fn get_filter_properties(id: Ulid, name: &str, volume: f32, tx: Sender<()>) -> FilterProperties {
    FilterProperties {
        filter_id: id,
        filter_name: "Volume".into(),
        filter_nick: name.to_string(),
        filter_description: format!("pipecast/{}", name.to_string().to_lowercase().replace(" ", "-")),
        app_id: "com.frostycoolslug".to_string(),
        app_name: "pipecast".to_string(),
        linger: false,
        callback: Box::new(VolumeFilter { volume }),
        ready_sender: tx,
    }
}

struct PassThroughFilter {}
impl FilterHandler for crate::PassThroughFilter {
    fn get_properties(&self) -> Vec<FilterProperty> {
        vec![]
    }

    fn get_property(&self, _: u32) -> FilterProperty {
        panic!("Attempted to get non-existent property");
    }

    fn set_property(&mut self, _: u32, _: FilterValue) {
        panic!("Attempted to set non-existent property");
    }

    fn process_samples(&self, inputs: Vec<&mut [f32]>, mut outputs: Vec<&mut [f32]>) {
        for (i, input) in inputs.iter().enumerate() {
            if input.is_empty() || outputs[i].is_empty() {
                continue;
            }
            outputs[i].copy_from_slice(input);
        }
    }
}

fn get_pass_through_properties(id: Ulid, name: &str, tx: Sender<()>) -> FilterProperties {
    FilterProperties {
        filter_id: id,
        filter_name: "Pass".into(),
        filter_nick: name.to_string(),
        filter_description: format!("pipecast/{}", name.to_string().to_lowercase().replace(" ", "-")),
        app_id: "com.frostycoolslug".to_string(),
        app_name: "pipecast".to_string(),
        linger: false,
        callback: Box::new(PassThroughFilter {}),
        ready_sender: tx,
    }
}
