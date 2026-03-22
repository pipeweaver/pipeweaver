mod cli;
use anyhow::Result;
use clap::Parser;
use pipeweaver_ipc::commands::{APICommand, DaemonCommand, DaemonRequest};
use pipeweaver_shared::AppDefinition;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let msg = match cli.command {
        Some(command) => match command {
            cli::Commands::Node { command } => handle_node_command(command),
            cli::Commands::App { command } => handle_app_command(command),
            cli::Commands::Route { command } => handle_route_command(command),
            cli::Commands::Daemon { command } => handle_daemon_command(command),
        },
        None => return Ok(()),
    };
    println!("PipeWeaver message: {msg:#?}");

    Ok(())
}

fn handle_node_command(cmd: cli::NodeCommands) -> DaemonRequest {
    use cli::NodeCommands::*;
    use cli::NodeIdCommands as IdCmd;
    let api_cmd = match cmd {
        Create { node_type, name } => APICommand::CreateNode(node_type, name),
        Edit {
            name: src_name,
            command,
        } => match command {
            IdCmd::Rename { name } => APICommand::RenameNodeByName(src_name, name),
            IdCmd::SetColour { colour } => APICommand::SetNodeColourByName(src_name, colour),
            IdCmd::Remove => APICommand::RemoveNodeByName(src_name),
            IdCmd::SetVolume { mix, volume } => APICommand::SetVolumeByName(src_name, mix, volume),
            IdCmd::SetSourceVolumeLinked { linked } => {
                APICommand::SetSourceVolumeLinkedByName(src_name, linked)
            }
            IdCmd::SetTargetMix { mix } => APICommand::SetTargetMixByName(src_name, mix),
            IdCmd::AddSourceMuteTarget { target } => {
                APICommand::AddSourceMuteTargetByName(src_name, target)
            }
            IdCmd::DelSourceMuteTarget { target } => {
                APICommand::DelSourceMuteTargetByName(src_name, target)
            }
            IdCmd::AddMuteTargetNode { target, node } => {
                APICommand::AddMuteTargetNodeByNames(src_name, target, node)
            }
            IdCmd::DelMuteTargetNode { target, node } => {
                APICommand::DelMuteTargetNodeByNames(src_name, target, node)
            }
            IdCmd::ClearMuteTargetNodes { target } => {
                APICommand::ClearMuteTargetNodesByName(src_name, target)
            }
            IdCmd::SetTargetMuteState { state } => {
                APICommand::SetTargetMuteStatesByName(src_name, state)
            }
            IdCmd::AttachPhysicalNode { device } => {
                APICommand::AttachPhysicalNodeByName(src_name, device)
            }
            IdCmd::RemovePhysicalNode { index } => {
                APICommand::RemovePhysicalNodeByName(src_name, index)
            }
            IdCmd::SetOrderGroup { group } => APICommand::SetOrderGroupByName(src_name, group),
            IdCmd::SetOrder { order } => APICommand::SetOrderByName(src_name, order),
        },
    };
    DaemonRequest::Pipewire(api_cmd)
}

fn handle_app_command(cmd: cli::AppCommands) -> DaemonRequest {
    use cli::AppCommands::*;
    let api_cmd = match cmd {
        SetRoute {
            device_type,
            process,
            name,
            target,
        } => APICommand::SetApplicationRouteByName(
            AppDefinition {
                device_type,
                process,
                name,
            },
            target,
        ),
        ClearRoute {
            device_type,
            process,
            name,
        } => APICommand::ClearApplicationRoute(AppDefinition {
            device_type,
            process,
            name,
        }),
        SetTransientRoute { process_id, target } => {
            APICommand::SetTransientApplicationRouteByName(process_id, target)
        }
        ClearTransientRoute { process_id } => {
            APICommand::ClearTransientApplicationRoute(process_id)
        }
        SetVolume { process_id, volume } => APICommand::SetApplicationVolume(process_id, volume),
        SetMute { process_id, muted } => APICommand::SetApplicationMute(process_id, muted),
    };
    DaemonRequest::Pipewire(api_cmd)
}

fn handle_route_command(cmd: cli::RouteCommands) -> DaemonRequest {
    use cli::RouteCommands::*;
    let api_cmd = match cmd {
        Set {
            source,
            target,
            enabled,
        } => APICommand::SetRouteByNames(source, target, enabled),
    };
    DaemonRequest::Pipewire(api_cmd)
}

fn handle_daemon_command(cmd: cli::DaemonCommands) -> DaemonRequest {
    use cli::DaemonCommands::*;
    let daemon_cmd = match cmd {
        SetAutoStart { enabled } => DaemonCommand::SetAutoStart(enabled),
        SetAudioQuantum { quantum } => DaemonCommand::SetAudioQuantum(quantum),
        OpenInterface => DaemonCommand::OpenInterface,
        ResetAudio => DaemonCommand::ResetAudio,
    };
    DaemonRequest::Daemon(daemon_cmd)
}
