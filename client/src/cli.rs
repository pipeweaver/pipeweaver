use clap::{Parser, Subcommand};
use pipeweaver_shared::{
    Colour, DeviceType, Mix, MuteState, MuteTarget, NodeType, OrderGroup, Quantum,
};

/// PipeWeaver CLI
#[derive(Parser, Debug)]
#[command(about, version, author)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Display status after command.
    #[arg(long)]
    pub status: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum Commands {
    /// Node-related commands (by node ID or for creation)
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },
    /// Route-related commands
    Route {
        #[command(subcommand)]
        command: RouteCommands,
    },
    /// Application-related commands
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
    /// Daemon/Device-related commands
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum NodeCommands {
    /// Create a new node
    Create {
        #[arg(value_enum)]
        node_type: NodeType,
        name: String,
    },
    /// Operate on an existing node by ID
    Edit {
        name: String,
        #[command(subcommand)]
        command: NodeIdCommands,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum NodeIdCommands {
    Rename {
        name: String,
    },
    SetColour {
        colour: Colour,
    },
    Remove,
    SetVolume {
        /// Volume as a percentage (0-100)
        #[arg(value_parser = percent_value)]
        volume: u8,

        /// The Mix to be adjusted
        mix: Option<Mix>,
    },
    SetSourceVolumeLinked {
        linked: bool,
    },
    SetTargetMix {
        #[arg(value_enum)]
        mix: Mix,
    },
    AddSourceMuteTarget {
        #[arg(value_enum)]
        target: MuteTarget,
    },
    DelSourceMuteTarget {
        #[arg(value_enum)]
        target: MuteTarget,
    },
    AddMuteTargetNode {
        #[arg(value_enum)]
        target: MuteTarget,
        node: String,
    },
    DelMuteTargetNode {
        #[arg(value_enum)]
        target: MuteTarget,
        node: String,
    },
    ClearMuteTargetNodes {
        #[arg(value_enum)]
        target: MuteTarget,
    },
    SetTargetMuteState {
        #[arg(value_enum)]
        state: MuteState,
    },
    AttachPhysicalNode {
        device: u32,
    },
    RemovePhysicalNode {
        index: usize,
    },
    SetOrderGroup {
        #[arg(value_enum)]
        group: OrderGroup,
    },
    SetOrder {
        order: u8,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum RouteCommands {
    Set {
        source: String,
        target: String,
        enabled: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum AppCommands {
    SetRoute {
        device_type: DeviceType,
        process: String,
        name: String,
        target: String,
    },
    ClearRoute {
        device_type: DeviceType,
        process: String,
        name: String,
    },
    SetTransientRoute {
        process_id: u32,
        target: String,
    },
    ClearTransientRoute {
        process_id: u32,
    },
    SetVolume {
        process_id: u32,
        /// Volume as a percentage (0-100)
        #[arg(value_parser = percent_value)]
        volume: u8,
    },
    SetMute {
        process_id: u32,
        muted: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum DaemonCommands {
    SetAutoStart {
        enabled: bool,
    },
    SetAudioQuantum {
        #[arg(value_enum)]
        quantum: Quantum,
    },
    OpenInterface,
    ResetAudio,
}

// Example argument validation function (if needed)
pub fn percent_value(s: &str) -> Result<u8, String> {
    let value = s
        .parse::<u8>()
        .map_err(|_| "Value must be between 0 and 100".to_string())?;
    if value > 100 {
        return Err("Value must be lower than 100".to_string());
    }
    Ok(value)
}
