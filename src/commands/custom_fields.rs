mod create;
mod delete;
mod get;
mod list;
mod update;
use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};
use clap::{Args, Subcommand};
#[derive(Debug, Args)]
pub struct CustomFieldArgs {
    #[command(subcommand)]
    pub command: CustomFieldCommand,
}
#[derive(Debug, Subcommand)]
pub enum CustomFieldCommand {
    /// List board custom-field definitions.
    List(list::ListArgs),
    /// Get board custom-field definitions.
    Get(get::GetArgs),
    /// Create board custom-field definitions.
    Create(create::CreateArgs),
    /// Update board custom-field definitions.
    Update(update::UpdateArgs),
    /// Permanently delete board custom-field definitions.
    Delete(delete::DeleteArgs),
    /// Manage custom-field dropdown items.
    Item(super::custom_field_items::CustomFieldItemArgs),
}
pub(crate) async fn dispatch(
    command: CustomFieldCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        CustomFieldCommand::List(args) => list::execute(args, factory, store).await,
        CustomFieldCommand::Get(args) => get::execute(args, factory, store).await,
        CustomFieldCommand::Create(args) => create::execute(args, factory, store).await,
        CustomFieldCommand::Update(args) => update::execute(args, factory, store).await,
        CustomFieldCommand::Delete(args) => {
            delete::execute(args, factory, store, confirmation).await
        }
        CustomFieldCommand::Item(args) => {
            super::custom_field_items::dispatch(args.command, factory, store, confirmation).await
        }
    }
}
pub(crate) fn non_empty(value: &str) -> Result<String, String> {
    let value = crate::client::trim_javascript_whitespace(value);
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        crate::client::custom_fields::validate_custom_field_identifier(value)
            .map_err(str::to_owned)?;
        Ok(value.to_owned())
    }
}
fn field_type(value: &str) -> Result<crate::client::custom_fields::CustomFieldType, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|_| {
        "expected text, number, date, dropdown, dropdownMultiSelect, checkbox, currency or stringtemplate".to_owned()
    })
}
fn settings(value: &str) -> Result<crate::client::custom_fields::CustomFieldSettings, String> {
    let value: serde_json::Value =
        serde_json::from_str(value).map_err(|_| "expected a settings JSON object".to_owned())?;
    if !value.is_object() {
        return Err("expected a settings JSON object".to_owned());
    }
    let settings: crate::client::custom_fields::CustomFieldSettings = serde_json::from_value(value)
        .map_err(|_| {
            "expected custom-field settings with only the known, correctly typed properties"
                .to_owned()
        })?;
    settings
        .validate_dropdown_item_ids()
        .map_err(str::to_owned)?;
    Ok(settings)
}
pub(crate) fn exact_json(value: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(value).map_err(|_| "expected a JSON value".to_owned())
}
fn type_name(value: crate::client::custom_fields::CustomFieldType) -> String {
    use crate::client::custom_fields::CustomFieldType::*;
    match value {
        Text => "text",
        Number => "number",
        Date => "date",
        Dropdown => "dropdown",
        DropdownMultiSelect => "dropdownMultiSelect",
        Checkbox => "checkbox",
        Currency => "currency",
        Stringtemplate => "stringtemplate",
    }
    .to_owned()
}
fn detail(
    value: crate::client::custom_fields::CustomFieldDocument,
) -> crate::command_result::CustomFieldDetail {
    use crate::command_result::{CustomFieldDetail, CustomFieldDropdownItem, CustomFieldSettings};
    CustomFieldDetail {
        custom_field_id: value.custom_field_id,
        board_ids: value.board_ids,
        name: value.name,
        field_type: type_name(value.field_type),
        settings: CustomFieldSettings {
            jira_time_field: value.settings.jira_time_field.flatten(),
            jira_estimate_field_id: value.settings.jira_estimate_field_id.flatten(),
            jira_estimate_unit: value.settings.jira_estimate_unit.flatten(),
            currency_code: value.settings.currency_code.flatten(),
            dropdown_items: value.settings.dropdown_items.flatten().map(|items| {
                items
                    .into_iter()
                    .map(|item| CustomFieldDropdownItem {
                        item_id: item.item_id,
                        name: item.name,
                    })
                    .collect()
            }),
            stringtemplate_format: value.settings.stringtemplate_format.flatten(),
            stringtemplate_separator: value.settings.stringtemplate_separator.flatten(),
        },
        show_on_card: value.show_on_card,
        automatically_on_card: value.automatically_on_card,
        always_on_card: value.always_on_card,
        show_label_on_mini_card: value.show_label_on_mini_card,
        show_sum_at_top_of_list: value.show_sum_at_top_of_list,
        created_at: value.created_at,
        modified_at: value.modified_at,
        card_kinds: value.card_kinds,
        read_only: value.read_only,
        admin_only: value.admin_only,
        sort: value.sort,
    }
}

#[cfg(test)]
mod tests;
