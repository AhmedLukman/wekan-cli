use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    wekan_cli::run().await
}
