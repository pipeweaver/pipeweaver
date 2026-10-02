mod cli;

use anyhow::{Error, Result, bail};
use clap::Parser;
use directories::BaseDirs;
use pipeweaver_ipc::client::Client;
use pipeweaver_ipc::clients::ipc::IPCClient;
use pipeweaver_ipc::clients::web::WebClient;
use pipeweaver_ipc::commands::{
    APICommand, DaemonCommand, DaemonRequest, DaemonResponse, DaemonStatus, PWCommandResponse,
};
use pipeweaver_shared::AppDefinition;
use std::path::PathBuf;
use std::{env, fs};
use ulid::Ulid;

const APP_NAME: &str = "PipeWeaver";
const APP_NAME_ID: &str = "pipeweaver";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Attempt an IPC connection
    let mut client: Box<dyn Client> = if let Some(url) = cli.use_http {
        Box::new(WebClient::new(format!("{url}/api/command")))
    } else {
        let path = get_socket_path()?;
        Box::new(IPCClient::connect(path).await?)
    };

    // Poll the Status
    let status = client.get_status().await?;

    let msg = cli.command.map(|command| match command {
        cli::Commands::Node { command } => handle_node_command(&status, command),
        cli::Commands::App { command } => handle_app_command(&status, command),
        cli::Commands::Route { command } => handle_route_command(&status, command),
        cli::Commands::Daemon { command } => handle_daemon_command(&status, command),
    });

    if let Some(msg) = msg {
        let response = client.send(&msg?).await?;
        match response {
            DaemonResponse::Ok => {}
            DaemonResponse::Err(e) => {
                bail!("A General Error Occurred: {}", e);
            }
            DaemonResponse::Pipewire(e) => match e {
                PWCommandResponse::Ok => {}
                PWCommandResponse::Id(e) => {
                    println!("Received: {}", e);
                }
                PWCommandResponse::Err(e) => bail!("{}", e),
            },
            _ => bail!("Unexpected Response"),
        }
    }

    if cli.status {
        // Ok, convert this object to json for outputs
        let out = serde_json::to_string_pretty(&status)?;
        println!("{}", out);
    }

    Ok(())
}

fn handle_node_command(status: &DaemonStatus, cmd: cli::NodeCommands) -> Result<DaemonRequest> {
    use cli::NodeCommands::*;
    use cli::NodeIdCommands as IdCmd;
    let api_cmd = match cmd {
        Create { node_type, name } => APICommand::CreateNode(node_type, name),
        Edit {
            name: src_name,
            command,
        } => {
            let id = get_node_id_by_name(status, &src_name)?;
            match command {
                IdCmd::Rename { name } => APICommand::RenameNode(id, name),
                IdCmd::SetColour { colour } => APICommand::SetNodeColour(id, colour),
                IdCmd::Remove => APICommand::RemoveNode(id),
                IdCmd::SetVolume { mix, volume } => APICommand::SetVolume(id, mix, volume),
                IdCmd::SetSourceVolumeLinked { linked } => {
                    APICommand::SetSourceVolumeLinked(id, linked)
                }
                IdCmd::SetTargetMix { mix } => APICommand::SetTargetMix(id, mix),
                IdCmd::AddSourceMuteTarget { target } => {
                    APICommand::AddSourceMuteTarget(id, target)
                }
                IdCmd::DelSourceMuteTarget { target } => {
                    APICommand::DelSourceMuteTarget(id, target)
                }
                IdCmd::AddMuteTargetNode { target, node } => {
                    let node = get_node_id_by_name(status, &node)?;
                    APICommand::AddMuteTargetNode(id, target, node)
                }
                IdCmd::DelMuteTargetNode { target, node } => {
                    let node = get_node_id_by_name(status, &node)?;
                    APICommand::DelMuteTargetNode(id, target, node)
                }
                IdCmd::ClearMuteTargetNodes { target } => {
                    APICommand::ClearMuteTargetNodes(id, target)
                }
                IdCmd::SetTargetMuteState { state } => APICommand::SetTargetMuteState(id, state),
                IdCmd::AttachPhysicalNode { device } => APICommand::AttachPhysicalNode(id, device),
                IdCmd::RemovePhysicalNode { index } => APICommand::RemovePhysicalNode(id, index),
                IdCmd::SetOrderGroup { group } => APICommand::SetOrderGroup(id, group),
                IdCmd::SetOrder { order } => APICommand::SetOrder(id, order),
            }
        }
        _ => {
            bail!("Invalid Command");
        }
    };
    Ok(DaemonRequest::Pipewire(api_cmd))
}

fn handle_app_command(status: &DaemonStatus, cmd: cli::AppCommands) -> Result<DaemonRequest> {
    use cli::AppCommands::*;
    let api_cmd = match cmd {
        SetRoute {
            device_type,
            process,
            name,
            target,
        } => {
            let target = get_node_id_by_name(status, &target)?;
            let definition = AppDefinition {
                device_type,
                process,
                name,
            };

            APICommand::SetApplicationRoute(definition, target)
        }
        ClearRoute {
            device_type,
            process,
            name,
        } => {
            let definition = AppDefinition {
                device_type,
                process,
                name,
            };

            APICommand::ClearApplicationRoute(definition)
        }
        SetTransientRoute { process_id, target } => {
            let target = get_node_id_by_name(status, &target)?;
            APICommand::SetTransientApplicationRoute(process_id, target)
        }
        ClearTransientRoute { process_id } => {
            APICommand::ClearTransientApplicationRoute(process_id)
        }
        SetVolume { process_id, volume } => APICommand::SetApplicationVolume(process_id, volume),
        SetMute { process_id, muted } => APICommand::SetApplicationMute(process_id, muted),
    };
    Ok(DaemonRequest::Pipewire(api_cmd))
}

fn handle_route_command(status: &DaemonStatus, cmd: cli::RouteCommands) -> Result<DaemonRequest> {
    use cli::RouteCommands::*;
    let api_cmd = match cmd {
        Set {
            source,
            target,
            enabled,
        } => {
            let source = get_node_id_by_name(status, &source)?;
            let target = get_node_id_by_name(status, &target)?;
            APICommand::SetRoute(source, target, enabled)
        }
        Toggle { source, target } => {
            let source = get_node_id_by_name(status, &source)?;
            let target = get_node_id_by_name(status, &target)?;
            APICommand::ToggleRoute(source, target)
        }
    };
    Ok(DaemonRequest::Pipewire(api_cmd))
}

fn handle_daemon_command(_: &DaemonStatus, cmd: cli::DaemonCommands) -> Result<DaemonRequest> {
    use cli::DaemonCommands::*;
    let daemon_cmd = match cmd {
        SetAutoStart { enabled } => DaemonCommand::SetAutoStart(enabled),
        SetUseBrowser { enabled } => DaemonCommand::SetUseBrowser(enabled),
        SetAudioQuantum { quantum } => DaemonCommand::SetAudioQuantum(Some(quantum)),
        ClearAudioQuantum => DaemonCommand::SetAudioQuantum(None),
        OpenInterface => DaemonCommand::OpenInterface,
        ResetAudio => DaemonCommand::ResetAudio,
    };
    Ok(DaemonRequest::Daemon(daemon_cmd))
}

pub fn get_socket_path() -> Result<PathBuf> {
    let path = BaseDirs::new()
        .and_then(|base| base.runtime_dir().map(|p| p.to_path_buf()))
        .map(Ok::<PathBuf, Error>)
        .unwrap_or_else(|| {
            let tmp_dir = env::temp_dir().join(APP_NAME);
            if !tmp_dir.exists() {
                fs::create_dir_all(&tmp_dir)?;
            }
            Ok(tmp_dir)
        })?;

    let socket_path = path.join(format!("{}.socket", APP_NAME_ID));
    Ok(socket_path)
}

fn get_node_id_by_name(status: &DaemonStatus, name: &str) -> Result<Ulid> {
    for device in &status.audio.profile.devices.sources.physical_devices {
        if device.description.name == name {
            return Ok(device.description.id);
        }
    }
    for device in &status.audio.profile.devices.sources.virtual_devices {
        if device.description.name == name {
            return Ok(device.description.id);
        }
    }
    for device in &status.audio.profile.devices.targets.physical_devices {
        if device.description.name == name {
            return Ok(device.description.id);
        }
    }
    for device in &status.audio.profile.devices.targets.virtual_devices {
        if device.description.name == name {
            return Ok(device.description.id);
        }
    }

    // This name wasn't found, so return none
    bail!("Node name {} not Found", name);
}
