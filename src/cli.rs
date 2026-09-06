use clap::Parser;

use crate::{commands::RootCommand, output::OutputFormat};

#[derive(Debug, Parser)]
#[command(
    name = "wekan",
    version,
    about = "Agent-first, human-friendly command-line client for Wekan",
    arg_required_else_help = true,
    after_help = "Examples:\n  wekan profile add local http://localhost:3000 --use\n  wekan auth login --username alice\n  wekan --output json board list\n  wekan card get card-id --board board-id --list list-id"
)]
pub struct Cli {
    /// Wekan server base URL. Initializes or verifies the selected profile, or identifies an orphaned credential for local-only logout.
    #[arg(long, global = true, help_heading = "Global options")]
    pub server: Option<String>,

    /// Named local server profile. Overrides environment and active selection.
    #[arg(
        long = "profile",
        global = true, help_heading = "Global options",
        value_parser = crate::config::profiles::parse_profile_name
    )]
    pub profile_name: Option<String>,

    /// Output format.
    #[arg(long, global = true, help_heading = "Global options", value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// Permit plaintext HTTP for a non-loopback server.
    #[arg(long, global = true, help_heading = "Global options")]
    pub allow_insecure_http: bool,

    #[command(subcommand)]
    pub command: RootCommand,
}
