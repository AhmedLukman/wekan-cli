use clap::Parser;

use crate::{commands::RootCommand, output::OutputFormat};

#[derive(Debug, Parser)]
#[command(
    name = "wekan",
    version,
    about = "Agent-first, human-friendly command-line client for Wekan",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Wekan server base URL. Overrides WEKAN_URL.
    #[arg(long, global = true, env = "WEKAN_URL")]
    pub server: Option<String>,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// Permit plaintext HTTP for a non-loopback server.
    #[arg(long, global = true)]
    pub allow_insecure_http: bool,

    #[command(subcommand)]
    pub command: RootCommand,
}
