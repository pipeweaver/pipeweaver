use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::primary_worker::ManagerMessage;
use anyhow::{anyhow, Error, Result};
use enum_map::EnumMap;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, error};
use pipecast_ipc::commands::{AudioConfiguration, PipewireCommandResponse};
use pipecast_pipewire::tokio::sync::oneshot::Receiver;
use pipecast_pipewire::LinkType::{Filter, UnmanagedNode};
use pipecast_pipewire::{
    MediaClass, PipecastNode,
    PipewireMessage, PipewireRunner,
};
use pipecast_profile::{PhysicalDeviceDescriptor, Profile};
use pipecast_shared::Mix;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use ulid::Ulid;

#[derive(Copy, Clone)]
pub(crate) struct Waker {
    pub(crate) id: Ulid,
    node_id: u32,
    class: MediaClass,
    created: Instant,
}

pub(crate) struct PipewireManager {
    command_receiver: mpsc::Receiver<ManagerMessage>,

    pub(crate) pipewire: PipewireRunner,

    pub(crate) profile: Profile,
    pub(crate) source_map: HashMap<Ulid, EnumMap<Mix, Ulid>>,
    pub(crate) target_map: HashMap<Ulid, Ulid>,

    // Maps the connection of a PassThrough filter to a Physical Source id
    pub(crate) physical_source: HashMap<Ulid, Vec<u32>>,
    pub(crate) physical_target: HashMap<Ulid, Vec<u32>>,

    pub(crate) wake_filters: HashMap<Ulid, Waker>,

    node_list: Vec<PipecastNode>,
}

impl PipewireManager {
    pub fn new(command_receiver: mpsc::Receiver<ManagerMessage>) -> Self {
        debug!("Establishing Connection to Pipewire..");
        let manager = PipewireRunner::new().unwrap();

        Self {
            command_receiver,
            pipewire: manager,

            profile: Profile::base_settings(),

            source_map: HashMap::default(),
            target_map: HashMap::default(),

            physical_source: HashMap::default(),
            physical_target: HashMap::default(),

            wake_filters: HashMap::default(),

            node_list: Default::default(),
        }
    }

    async fn get_config(&self) -> AudioConfiguration {
        AudioConfiguration {
            profile: self.profile.clone(),
        }
    }

    pub async fn run(&mut self) {
        debug!("[Pipewire Runner] Starting Event Loop");

        debug!("[Pipewire Runner] Waiting for Pipewire to Calm..");
        sleep(Duration::from_secs(1)).await;

        let (tx, rx) = oneshot::channel();
        let _ = self
            .pipewire
            .send_message(PipewireMessage::GetUsableNodes(tx));
        if let Ok(nodes) = rx.await {
            self.node_list = nodes;
        }

        debug!("Loading Profile");
        let mut wakers: FuturesUnordered<Receiver<Ulid>> = FuturesUnordered::new();
        if let Err(e) = self.load_profile().await {
            error!("Error Loading Profile: {}", e);
        }


        loop {
            select!(
                Some(command) = self.command_receiver.recv() => {
                    match command {
                        ManagerMessage::Execute(command, tx) => {
                            let result: Result<(), Error> = match command {
                                // PipewireCommand::SetVolume(node_type, id, mix, volume) => {
                                //     self.set_volume(node_type, id, mix, volume).await
                                // }
                                // PipewireCommand::SetMuteState(node_type, id, target, state) => {
                                //     //self.set_mute_state(node_type, id, target, state).await
                                //     //Ok(())
                                // }
                                _ => {
                                    debug!("Received Command: {:?}", command);
                                    Err(anyhow!("Command Not Implemented"))
                                }
                            };

                            // Map the result to a PW Response and send it
                            let _ = tx.send(match result {
                                Ok(_) => PipewireCommandResponse::Ok,
                                Err(e) => PipewireCommandResponse::Err(e.to_string())
                            });
                        }
                        ManagerMessage::GetConfig(tx) => {
                            let _ = tx.send(self.get_config().await);
                        }
                    }
                }
                Some(result) = wakers.next() => {
                    if let Ok(id) = result {
                        if let Some(waker) = self.wake_filters.get(&id) {
                            debug!("[{}] Device Woke up In {:?}, attaching to tree..", id, waker.created.elapsed());

                            let (source, destination) = match waker.class {
                                MediaClass::Source => (UnmanagedNode(waker.node_id), Filter(id)),
                                MediaClass::Sink => (Filter(id), UnmanagedNode(waker.node_id)),
                                MediaClass::Duplex => panic!("Unexpected Duplex!")
                            };

                            // Attach the Original Node to Tree...
                            let _ = self.pipewire.send_message(PipewireMessage::CreateDeviceLink(source, destination));

                            // 1 the links for the wake node...
                            let (source, destination) = match waker.class {
                                MediaClass::Source => (UnmanagedNode(waker.node_id), Filter(waker.id)),
                                MediaClass::Sink => (Filter(waker.id), UnmanagedNode(waker.node_id)),
                                MediaClass::Duplex => panic!("Unexpected Duplex!")
                            };
                            let _ = self.pipewire.send_message(PipewireMessage::RemoveDeviceLink(source, destination));
                            let _ = self.pipewire.send_message(PipewireMessage::RemoveFilterNode(waker.id));
                        }
                    }
                }
            );
        }
    }


    // Keeping this for now :D
    fn locate_physical_node_id(
        &self,
        device: &PhysicalDeviceDescriptor,
        input: bool,
    ) -> Option<u32> {
        debug!("Looking for Physical Device: {:?}", device);

        // This might look a little cumbersome, especially with the need to iterate three
        // times, HOWEVER, we have to check this in terms of accuracy. The name is the
        // specific location of a device on the USB / PCI-E / etc bus, which if we hit is
        // a guaranteed 100% 'this is our device' match.
        for node in &self.node_list {
            if device.name == node.name
                && ((input && node.inputs != 0) || (!input && node.outputs != 0))
            {
                debug!(
                    "Found Name Match {:?}, NodeId: {}",
                    device.name, node.node_id
                );
                return Some(node.node_id);
            }
        }

        // The description is *GENERALLY* unique, and represents how the device is displayed
        // in things like pavucontrol, Gnome's and KDE's audio settings, etc., but uniqueness
        // is less guaranteed here. This is often useful in situations where (for example)
        // the device is plugged into a different USB port, so it's name has changed
        if device.description.is_some() {
            for node in &self.node_list {
                if device.description == node.description
                    && ((input && node.inputs != 0) || (!input && node.outputs != 0))
                {
                    debug!(
                        "Found Description Match {:?}, NodeId: {}",
                        device.description, node.node_id
                    );
                    return Some(node.node_id);
                }
            }
        }

        // Finally, we'll check by nickname. In my experience this is very NOT unique, for
        // example, all my GoXLR nodes have their nickname as 'GoXLR', I'm at least slightly
        // skeptical whether I should even track this due to the potential for false
        // positives, but it's here for now.
        if device.nickname.is_some() {
            for node in &self.node_list {
                if device.nickname == node.nickname
                    && ((input && node.inputs != 0) || (!input && node.outputs != 0))
                {
                    debug!(
                        "Found Nickname Match {:?}, NodeId: {}",
                        device.nickname, node.node_id
                    );
                    return Some(node.node_id);
                }
            }
        }
        debug!("Device Not Found: {:?}", device);

        None
    }
}

pub async fn run_pipewire_manager(command_receiver: mpsc::Receiver<ManagerMessage>) {
    let mut manager = PipewireManager::new(command_receiver);
    manager.run().await;
}
