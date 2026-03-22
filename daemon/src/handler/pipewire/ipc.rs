use crate::handler::pipewire::components::application::ApplicationManagement;
use crate::handler::pipewire::components::load_profile::LoadProfile;
use crate::handler::pipewire::components::mute::MuteManager;
use crate::handler::pipewire::components::node::NodeManagement;
use crate::handler::pipewire::components::physical::PhysicalDevices;
use crate::handler::pipewire::components::routing::RoutingManagement;
use crate::handler::pipewire::components::volume::VolumeManager;
use crate::handler::pipewire::manager::PipewireManager;
use anyhow::{Error, bail};
use log::debug;
use pipeweaver_ipc::commands::{APICommand, PWCommandResponse};
use pipeweaver_shared::MuteState::{Muted, Unmuted};
use pipeweaver_shared::{Mix, NodeType};

type Cmd = APICommand;
type Resp = PWCommandResponse;
pub(crate) trait IPCHandler {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error>;
}

impl IPCHandler for PipewireManager {
    async fn handle_command(&mut self, command: Cmd) -> Result<Resp, Error> {
        match command {
            Cmd::CreateNode(node_type, id) => self.node_new(node_type, id).await.map(Resp::Id),

            Cmd::RenameNode(id, new) => self.node_rename(id, new).await.map(|_| Resp::Ok),
            Cmd::RenameNodeByName(name, new) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.node_rename(id, new).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetNodeColour(id, colour) => {
                self.node_set_colour(id, colour).await.map(|_| Resp::Ok)
            }
            Cmd::SetNodeColourByName(name, colour) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.node_set_colour(id, colour).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::RemoveNode(id) => self.node_remove(id).await.map(|_| Resp::Ok),
            Cmd::RemoveNodeByName(name) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.node_remove(id).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetSourceVolume(id, mix, volume) => self
                .set_source_volume(id, mix, volume, true)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetTargetVolume(id, volume) => self
                .set_target_volume(id, volume, true)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetVolumeByName(name, mix, volume) => {
                let mix = if let Some(mix) = mix { mix } else { Mix::A };

                if let Some(id) = self.get_node_id_by_name(&name) {
                    if let Some(node_type) = self.get_node_type(id) {
                        if matches!(
                            node_type,
                            NodeType::PhysicalSource | NodeType::VirtualSource
                        ) {
                            self.set_source_volume(id, mix, volume, true)
                                .await
                                .map(|_| Resp::Ok)
                        } else {
                            self.set_target_volume(id, volume, true)
                                .await
                                .map(|_| Resp::Ok)
                        }
                    } else {
                        bail!("Node type for id {} not found", id);
                    }
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetSourceVolumeLinked(id, linked) => self
                .set_source_volume_linked(id, linked)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetSourceVolumeLinkedByName(name, linked) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.set_source_volume_linked(id, linked)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetTargetMix(target, mix) => self
                .routing_set_target_mix(target, mix)
                .await
                .map(|_| Resp::Ok),

            Cmd::SetTargetMixByName(name, mix) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.routing_set_target_mix(id, mix).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetRoute(source, target, enabled) => self
                .routing_set_route(source, target, enabled)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetRouteBySourceName(source_name, target, enabled) => {
                if let Some(source_id) = self.get_node_id_by_name(&source_name) {
                    self.routing_set_route(source_id, target, enabled)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", source_name);
                }
            }
            Cmd::SetRouteByTargetName(source, target_name, enabled) => {
                if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                    self.routing_set_route(source, target_id, enabled)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", target_name);
                }
            }
            Cmd::SetRouteByNames(source_name, target_name, enabled) => {
                if let Some(source_id) = self.get_node_id_by_name(&source_name) {
                    if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                        self.routing_set_route(source_id, target_id, enabled)
                            .await
                            .map(|_| Resp::Ok)
                    } else {
                        bail!("Target name {} not Found", target_name);
                    }
                } else {
                    bail!("Source name {} not Found", source_name);
                }
            }

            Cmd::AddSourceMuteTarget(id, target) => self
                .set_source_mute_state(id, target, Muted)
                .await
                .map(|_| Resp::Ok),
            Cmd::AddSourceMuteTargetByName(name, target) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.set_source_mute_state(id, target, Muted)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Source name {} not Found", name);
                }
            }

            Cmd::DelSourceMuteTarget(id, target) => self
                .set_source_mute_state(id, target, Unmuted)
                .await
                .map(|_| Resp::Ok),
            Cmd::DelSourceMuteTargetByName(name, target) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.set_source_mute_state(id, target, Unmuted)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::AddMuteTargetNode(id, target, target_id) => self
                .add_target_mute_node(id, target, target_id)
                .await
                .map(|_| Resp::Ok),

            Cmd::AddMuteTargetNodeBySourceName(source_name, target, target_id) => {
                if let Some(id) = self.get_node_id_by_name(&source_name) {
                    self.add_target_mute_node(id, target, target_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", source_name);
                }
            }
            Cmd::AddMuteTargetNodeByTargetName(id, target, target_name) => {
                if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                    self.add_target_mute_node(id, target, target_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", target_name);
                }
            }
            Cmd::AddMuteTargetNodeByNames(source_name, target, target_name) => {
                if let Some(id) = self.get_node_id_by_name(&source_name) {
                    if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                        self.add_target_mute_node(id, target, target_id)
                            .await
                            .map(|_| Resp::Ok)
                    } else {
                        bail!("Target name {} not Found", target_name);
                    }
                } else {
                    bail!("Source name {} not Found", source_name);
                }
            }

            Cmd::DelMuteTargetNode(id, target, target_id) => self
                .del_target_mute_node(id, target, target_id)
                .await
                .map(|_| Resp::Ok),
            Cmd::DelMuteTargetNodeBySourceName(source_name, target, target_id) => {
                if let Some(id) = self.get_node_id_by_name(&source_name) {
                    self.del_target_mute_node(id, target, target_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", source_name);
                }
            }
            Cmd::DelMuteTargetNodeByTargetName(id, target, target_name) => {
                if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                    self.del_target_mute_node(id, target, target_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", target_name);
                }
            }
            Cmd::DelMuteTargetNodeByNames(source_name, target, target_name) => {
                if let Some(id) = self.get_node_id_by_name(&source_name) {
                    if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                        self.del_target_mute_node(id, target, target_id)
                            .await
                            .map(|_| Resp::Ok)
                    } else {
                        bail!("Target name {} not Found", target_name);
                    }
                } else {
                    bail!("Source name {} not Found", source_name);
                }
            }

            Cmd::ClearMuteTargetNodes(id, target) => self
                .clear_target_mute_nodes(id, target)
                .await
                .map(|_| Resp::Ok),
            Cmd::ClearMuteTargetNodesByName(name, target) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.clear_target_mute_nodes(id, target)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetTargetMuteState(id, state) => self
                .set_target_mute_state(id, state)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetTargetMuteStatesByName(name, state) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.set_target_mute_state(id, state)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::AttachPhysicalNode(id, node_id) => {
                self.add_device_to_node(id, node_id).await.map(|_| Resp::Ok)
            }
            Cmd::AttachPhysicalNodeByName(name, node_id) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.add_device_to_node(id, node_id).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::RemovePhysicalNode(id, index) => self
                .remove_device_from_node(id, index)
                .await
                .map(|_| Resp::Ok),
            Cmd::RemovePhysicalNodeByName(name, index) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.remove_device_from_node(id, index)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }

            Cmd::SetApplicationRoute(definition, target_id) => self
                .set_application_target(definition, target_id)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetApplicationRouteByName(definition, target_name) => {
                if let Some(target_id) = self.get_node_id_by_name(&target_name) {
                    self.set_application_target(definition, target_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", target_name);
                }
            }

            Cmd::ClearApplicationRoute(definition) => self
                .clear_application_target(definition)
                .await
                .map(|_| Resp::Ok),

            Cmd::SetTransientApplicationRoute(id, route) => self
                .set_application_transient_target(id, route)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetTransientApplicationRouteByName(id, route_name) => {
                if let Some(route_id) = self.get_node_id_by_name(&route_name) {
                    self.set_application_transient_target(id, route_id)
                        .await
                        .map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", route_name);
                }
            }

            Cmd::ClearTransientApplicationRoute(id) => self
                .clear_application_transient_target(id)
                .await
                .map(|_| Resp::Ok),
            Cmd::SetApplicationVolume(id, volume) => {
                debug!("ERR?");
                self.set_application_volume(id, volume)
                    .await
                    .map(|_| Resp::Ok)
            }
            Cmd::SetApplicationMute(id, state) => {
                self.set_application_mute(id, state).await.map(|_| Resp::Ok)
            }

            Cmd::SetOrderGroup(id, group) => self.node_set_group(id, group).await.map(|_| Resp::Ok),
            Cmd::SetOrderGroupByName(name, group) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.node_set_group(id, group).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }
            Cmd::SetOrder(id, position) => {
                self.node_set_position(id, position).await.map(|_| Resp::Ok)
            }
            Cmd::SetOrderByName(name, position) => {
                if let Some(id) = self.get_node_id_by_name(&name) {
                    self.node_set_position(id, position).await.map(|_| Resp::Ok)
                } else {
                    bail!("Node name {} not Found", name);
                }
            }
        }
    }
}
