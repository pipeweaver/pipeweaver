use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Error;
use pipecast_ipc::commands::{PipeCastCommand, PipewireCommandResponse};
use pipecast_shared::MuteState::{Muted, Unmuted};

type Cmd = PipeCastCommand;
type Resp = PipewireCommandResponse;
pub(crate) trait IPCHandler {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error>;
}

impl IPCHandler for PipewireManager {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error> {
        match command {
            Cmd::CreateNode(node_type, id) => {
                self.node_new(node_type, id).await.map(Resp::Id)
            }
            Cmd::RemoveNode(id) => {
                self.node_remove(id).await.map(|_| Resp::Ok)
            }
            Cmd::SetSourceVolume(id, mix, volume) => {
                self.set_source_node_volume(id, mix, volume).await.map(|_| Resp::Ok)
            }
            Cmd::SetTargetVolume(id, volume) => {
                self.set_target_node_volume(id, volume).await.map(|_| Resp::Ok)
            }
            Cmd::SetTargetMix(target, mix) => {
                self.routing_set_target_mix(target, mix).await.map(|_| Resp::Ok)
            }
            Cmd::SetRoute(source, target, enabled) => {
                self.routing_set_route(source, target, enabled).await.map(|_| Resp::Ok)
            }
            Cmd::AddSourceMuteTarget(id, target) => {
                self.set_source_mute_state(id, target, Muted).await.map(|_| Resp::Ok)
            }
            Cmd::DelSourceMuteTarget(id, target) => {
                self.set_source_mute_state(id, target, Unmuted).await.map(|_| Resp::Ok)
            }
            Cmd::SetTargetMuteState(id, state) => {
                self.set_target_mute_state(id, state).await.map(|r| Resp::Ok)
            }
        }
    }
}