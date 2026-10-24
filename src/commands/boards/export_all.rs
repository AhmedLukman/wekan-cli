use crate::{
    client::{
        WekanClientFactory,
        exports::{ExportRequest, ExportTarget},
    },
    command_result::CommandSuccess,
    commands::exports::{self, DeliveryArgs},
    credentials::CredentialStore,
    error::AppError,
};
use clap::Args;
pub const FORMATS: [&str; 45] = [
    "wekan",
    "csv",
    "scsv",
    "tsv",
    "excel",
    "deck",
    "kanboard",
    "openproject",
    "github",
    "gitea",
    "forgejo",
    "gitlab",
    "trello",
    "jira",
    "asana",
    "zenkit",
    "markdown",
    "leo",
    "todotxt",
    "taskwarrior",
    "focalboard",
    "todoist",
    "planner",
    "meistertask",
    "obsidian",
    "linear",
    "ticktick",
    "clickup",
    "nullboard",
    "kanri",
    "pivotal",
    "redmine",
    "tasksorg",
    "monday",
    "businessmap",
    "superproductivity",
    "taiga",
    "vikunja",
    "wrike",
    "wrikeworkflow",
    "teamwork",
    "quire",
    "notion",
    "opml",
    "orgmode",
];
#[derive(Debug, Args)]
pub struct ExportAllArgs {
    #[arg(long,value_parser=FORMATS)]
    pub format: String,
    /// Board to include; repeat. Omit to export all eligible boards.
    #[arg(long="board",value_parser=exports::scope_id)]
    pub boards: Vec<String>,
    #[arg(long="field",value_parser=exports::BOARD_FIELDS)]
    pub fields: Vec<String>,
    /// Include attachment bytes where the native format supports them.
    #[arg(long,action=clap::ArgAction::Set)]
    pub attachments: Option<bool>,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}
pub(crate) async fn execute(
    args: ExportAllArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    if args.delivery.no_auth {
        return Err(AppError::invalid_input(
            "export-all requires authentication; omit --no-auth",
        ));
    }
    let request = ExportRequest {
        board_ids: args.boards,
        fields: args.fields,
        attachments: args.attachments,
        ..Default::default()
    };
    exports::execute(
        ExportTarget::AllBoards,
        args.format.parse().expect("validated format"),
        request,
        args.delivery,
        factory,
        store,
    )
    .await
}
