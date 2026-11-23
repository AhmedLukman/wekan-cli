use crate::{
    client::{WekanClientFactory, boards::import::ImportSource},
    command_result::{CommandSuccess, ImportUploadSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        imports::{self, ImportInput},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ImportUploadArgs {
    #[command(flatten)]
    pub input: ImportInput,
    /// Import source for the staged file.
    #[arg(long, value_parser = ImportSource::ALL.map(ImportSource::as_str))]
    pub source: String,
}

pub(crate) async fn execute(
    args: ImportUploadArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let input = imports::acquire_upload(&args.input.file).await?;
    let source = args
        .source
        .parse()
        .expect("parser validated the import source");
    let response = context
        .client()
        .upload_import(
            source,
            input.reader,
            input.length,
            context.record().token(),
            args.input.timeout,
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &Redactor::with_secret(context.record().token()),
                "import upload",
                "the import source was not found",
            )
        })?;
    Ok(CommandSuccess::ImportUploaded(ImportUploadSuccess {
        upload_id: response.upload_id,
        size: response.size,
    }))
}
