mod cli;

use anyhow::{Error, Result, bail};
use clap::Parser;
use directories::BaseDirs;
use interprocess::local_socket::GenericFilePath;
use interprocess::local_socket::ToFsName;
use interprocess::local_socket::tokio::prelude::LocalSocketStream;
use interprocess::local_socket::traits::tokio::Stream;
use pipeweaver_ipc::client::Client;
use pipeweaver_ipc::clients::ipc::ipc_client::IPCClient;
use pipeweaver_ipc::clients::ipc::ipc_socket::Socket;
use pipeweaver_ipc::clients::web::web_client::WebClient;
use pipeweaver_ipc::commands::{
    APICommand, DaemonCommand, DaemonRequest, DaemonResponse, PWCommandResponse,
};
use pipeweaver_shared::AppDefinition;
use std::path::PathBuf;
use std::{env, fs};

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

        let connection = LocalSocketStream::connect(path.to_fs_name::<GenericFilePath>()?).await?;
        let socket: Socket<DaemonResponse, DaemonRequest> = Socket::new(connection);
        Box::new(IPCClient::new(socket))
    };

    // Poll the Status
    let status = client.get_status().await?;

    let msg = cli.command.map(|command| match command {
        cli::Commands::Node { command } => handle_node_command(command),
        cli::Commands::App { command } => handle_app_command(command),
        cli::Commands::Route { command } => handle_route_command(command),
        cli::Commands::Daemon { command } => handle_daemon_command(command),
    });
    if let Some(msg) = msg {
        let response = client.send(&msg).await?;
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
        Toggle { source, target } => APICommand::ToggleRouteByNames(source, target),
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
