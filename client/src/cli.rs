use clap::Parser;

#[derive(Parser, Debug)]
#[command(about, version, author)]
#[command(arg_required_else_help = true)]
pub struct Cli {}