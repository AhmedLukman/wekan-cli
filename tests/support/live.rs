use super::{
    App, AppError, CapturingStore, Cli, FileProfileStore, FixedPassword, OutputFormat, Parser,
};
use serde_json::Value;

pub(crate) type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

pub(crate) async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let command = Cli::try_parse_from(argv).unwrap().command;
    let result = app.execute(command).await?;
    let human = wekan_cli::output::render_success(OutputFormat::Human, &result);
    assert!(!human.is_empty());
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}
