pub mod app;
pub mod cli;
pub mod client;
pub mod commands;
pub mod credentials;
pub mod error;
pub mod exit_code;
pub mod output;
pub mod redaction;

use std::{env, ffi::OsString, process::ExitCode};

use clap::{CommandFactory, Parser, error::ErrorKind};

use crate::{
    app::App,
    cli::Cli,
    error::AppError,
    output::{OutputFormat, render_error, render_success},
};

/// Parse the process arguments, execute one command, render its result, and
/// return the stable process exit code.
pub async fn run() -> ExitCode {
    let args: Vec<OsString> = env::args_os().collect();
    let requested_output = OutputFormat::detect_from_args(&args);

    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            let code = error.exit_code();
            let _ = error.print();
            return ExitCode::from(code as u8);
        }
        Err(error) => {
            let app_error = AppError::invalid_input(error.to_string());
            eprintln!("{}", render_error(requested_output, &app_error));
            return app_error.exit_code().into();
        }
    };

    let output_format = cli.output;
    let app = App::production(cli.server, cli.allow_insecure_http);

    match app.execute(cli.command).await {
        Ok(success) => {
            println!("{}", render_success(output_format, &success));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", render_error(output_format, &error));
            error.exit_code().into()
        }
    }
}

/// Return the generated Clap command for completion and contract tests.
pub fn command() -> clap::Command {
    Cli::command()
}
