pub mod app;
pub mod cli;
pub mod client;
pub mod command_result;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod error;
pub mod exit_code;
pub mod input;
pub mod output;
pub mod redaction;

use std::{
    env,
    ffi::OsString,
    io::{self, Write},
    process::ExitCode,
};

use clap::{CommandFactory, Parser, error::ErrorKind};

use crate::{
    app::App,
    cli::Cli,
    config::ServerSelection,
    error::AppError,
    output::{OutputFormat, render_error, write_success},
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
    if output_format == OutputFormat::Raw && !cli.command.supports_raw_output() {
        let error = AppError::invalid_input("--output raw is valid only with wekan api request");
        eprintln!("{}", render_error(output_format, &error));
        return error.exit_code().into();
    }
    let app = App::production_with_confirmation_interactivity(
        ServerSelection::new(cli.server, cli.profile_name, cli.allow_insecure_http),
        output_format != OutputFormat::Json,
    );

    match app.execute(cli.command).await {
        Ok(success) => {
            let mut stdout = io::stdout().lock();
            let mut stderr = io::stderr().lock();
            match write_success(output_format, success, &mut stdout, &mut stderr).await {
                Ok(exit_code) => exit_code,
                Err(error) => {
                    let _ = writeln!(stderr, "{}", render_error(output_format, &error));
                    error.exit_code().into()
                }
            }
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
