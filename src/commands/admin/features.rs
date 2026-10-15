use crate::{
    client::admin::people::{FeatureField, UpdateOrganizationFeatures, UpdateTeamFeatures},
    command_result::DestructiveOperation,
    commands::{
        authenticated::AuthenticatedContext,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::{CredentialMutation, CredentialStore},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
};
use clap::{Args, ValueEnum};
use time::OffsetDateTime;

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("feature_changes").required(true).multiple(true).args([
    "shared_templates", "propagate_members_to_boards", "sync_members_from_auth"
])))]
pub struct FeatureChanges {
    /// Allow sharing personal template boards with this resource.
    #[arg(long)]
    shared_templates: Option<bool>,
    /// Set the native member-propagation feature flag.
    #[arg(long)]
    propagate_members_to_boards: Option<bool>,
    /// Set the authentication-provider membership-sync feature flag.
    #[arg(long)]
    sync_members_from_auth: Option<bool>,
}
impl FeatureChanges {
    pub(super) fn organization_request(&self) -> UpdateOrganizationFeatures {
        UpdateOrganizationFeatures {
            shared_templates: self.shared_templates,
            propagate_members_to_boards: self.propagate_members_to_boards,
            sync_members_from_auth: self.sync_members_from_auth,
        }
    }
    pub(super) fn team_request(&self) -> UpdateTeamFeatures {
        UpdateTeamFeatures {
            shared_templates: self.shared_templates,
            propagate_members_to_boards: self.propagate_members_to_boards,
            sync_members_from_auth: self.sync_members_from_auth,
        }
    }
    pub(super) fn submitted_fields(&self) -> Vec<FeatureField> {
        [
            (self.shared_templates, FeatureField::SharedTemplates),
            (
                self.propagate_members_to_boards,
                FeatureField::PropagateMembersToBoards,
            ),
            (
                self.sync_members_from_auth,
                FeatureField::SyncMembersFromAuth,
            ),
        ]
        .into_iter()
        .filter_map(|(value, field)| value.map(|_| field))
        .collect()
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FeatureChoice {
    SharedTemplates,
    PropagateMembersToBoards,
    SyncMembersFromAuth,
}
impl From<FeatureChoice> for FeatureField {
    fn from(field: FeatureChoice) -> Self {
        match field {
            FeatureChoice::SharedTemplates => Self::SharedTemplates,
            FeatureChoice::PropagateMembersToBoards => Self::PropagateMembersToBoards,
            FeatureChoice::SyncMembersFromAuth => Self::SyncMembersFromAuth,
        }
    }
}

#[derive(Debug, Args)]
#[command(
    after_help = "Changes one feature on every resource of this kind. Requires interactive confirmation or --yes. Sends one native request; no client-side propagation or retries."
)]
pub struct UpdateAllArgs {
    /// Feature to change across the entire instance.
    #[arg(long, value_enum)]
    pub(super) field: FeatureChoice,
    /// Explicit true or false.
    #[arg(long, action = clap::ArgAction::Set)]
    pub(super) value: bool,
    #[command(flatten)]
    confirmation: ConfirmationArgs,
}

pub(super) fn identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must not be empty, whitespace, or dot-only".into())
    } else {
        Ok(value.into())
    }
}

pub(super) fn confirm_bulk<'a>(
    args: &UpdateAllArgs,
    provider: &dyn ConfirmationProvider,
    context: &AuthenticatedContext,
    store: &'a dyn CredentialStore,
    operation: DestructiveOperation,
) -> Result<Option<Box<dyn CredentialMutation + Send + 'a>>, AppError> {
    let guard = lock_credential_mutation(store, context.credential_target())?;
    let prompt = format!(
        "Run {} with {} set to {} across the entire instance?",
        operation.as_command(),
        args.field
            .to_possible_value()
            .expect("feature has a CLI value")
            .get_name(),
        args.value
    );
    if confirm_or_skip(
        &args.confirmation,
        provider,
        &ConfirmationRequest::new(operation, prompt),
    )? == ConfirmationDecision::Cancelled
    {
        return Ok(None);
    }
    let record = guard
        .load()
        .map_err(map_credential_load_error)?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::CredentialNotFound,
                "the credential used to prepare the bulk feature update is no longer stored; no HTTP request was sent",
                StableExitCode::Credential,
            )
        })?;
    if !record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing the bulk feature update; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    if record.token_expires() <= OffsetDateTime::now_utc() {
        return Err(AppError::new(
            ErrorCode::CredentialExpired,
            "the stored credential expired while preparing the bulk feature update; no HTTP request was sent",
            StableExitCode::Server,
        )
        .with_details(ErrorDetails {
            token_expires: Some(context.token_expires().into()),
            ..ErrorDetails::default()
        }));
    }
    Ok(Some(guard))
}
