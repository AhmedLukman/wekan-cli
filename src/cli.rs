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
    /// Wekan server base URL. Initializes or verifies the selected profile, or identifies an orphaned credential for local-only logout.
    #[arg(long, global = true)]
    pub server: Option<String>,

    /// Named local server profile. Overrides environment and active selection.
    #[arg(
        long = "profile",
        global = true,
        value_parser = crate::config::profiles::parse_profile_name
    )]
    pub profile_name: Option<String>,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Human)]
    pub output: OutputFormat,

    /// Permit plaintext HTTP for a non-loopback server.
    #[arg(long, global = true)]
    pub allow_insecure_http: bool,

    #[command(subcommand)]
    pub command: RootCommand,
}
