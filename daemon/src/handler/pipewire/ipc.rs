use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::Error;
use pipeweaver_ipc::commands::{APICommand, APICommandResponse};
use pipeweaver_shared::MuteState::{Muted, Unmuted};

type Cmd = APICommand;
type Resp = APICommandResponse;
pub(crate) trait IPCHandler {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error>;
}

impl IPCHandler for PipewireManager {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error> {
        match command {
            Cmd::CreateNode(node_type, id) => {
                self.node_new(node_type, id).await.map(Resp::Id)
            }
            Cmd::RenameNode(id, new) => {
                self.node_rename(id, new).await.map(|_| Resp::Ok)
            }
            Cmd::SetNodeColour(id, colour) => {
                self.node_set_colour(id, colour).await.map(|_| Resp::Ok)
            }
            Cmd::RemoveNode(id) => {
                self.node_remove(id).await.map(|_| Resp::Ok)
            }
            Cmd::SetSourceVolume(id, mix, volume) => {
                self.set_source_node_volume(id, mix, volume).await.map(|_| Resp::Ok)
            }
            Cmd::SetSourceVolumeLinked(id, linked) => {
                self.set_source_volume_linked(id, linked).await.map(|_| Resp::Ok)
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
            Cmd::AddMuteTargetNode(id, target, target_id) => {
                self.add_target_mute_node(id, target, target_id).await.map(|_| Resp::Ok)
            }
            Cmd::DelMuteTargetNode(id, target, target_id) => {
                self.del_target_mute_node(id, target, target_id).await.map(|_| Resp::Ok)
            }
            Cmd::ClearMuteTargetNodes(id, target) => {
                self.clear_target_mute_nodes(id, target).await.map(|_| Resp::Ok)
            }
            Cmd::SetTargetMuteState(id, state) => {
                self.set_target_mute_state(id, state).await.map(|_| Resp::Ok)
            }

            Cmd::AttachPhysicalNode(id, node_id) => {
                self.add_device_to_node(id, node_id).await.map(|_| Resp::Ok)
            }
            Cmd::RemovePhysicalNode(id, index) => {
                self.remove_device_from_node(id, index).await.map(|_| Resp::Ok)
            }
            
            Cmd::SetOrder(id, position) => {
                self.node_set_position(id, position).await.map(|_| Resp::Ok)
            }
        }
    }
}