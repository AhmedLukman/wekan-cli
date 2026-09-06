use clap::Args;

use crate::{
    client::{
        WekanClientFactory,
        cards::{
            CardCustomField, CardCustomFieldValue, CardDependency, CardDocument, CardLocation,
            CardPoker, CardSticker, CardStickerHighlight, CardVote,
        },
    },
    command_result::{
        CardDetail, CardDetailCustomField, CardDetailCustomFieldValue, CardDetailDependency,
        CardDetailLocation, CardDetailPoker, CardDetailSticker, CardDetailStickerHighlight,
        CardDetailVote, CommandSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty)]
    pub list_id: String,

    /// Wekan card ID.
    #[arg(value_parser = non_empty)]
    pub card_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let card = context
        .client()
        .card(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "card lookup",
                "the requested Wekan card was not found in that board list",
            )
        })?;
    Ok(CommandSuccess::CardShown(Box::new(card_detail(card))))
}

fn card_detail(card: CardDocument) -> CardDetail {
    CardDetail {
        card_id: card.card_id,
        title: card.title,
        archived: card.archived,
        archived_at: card.archived_at,
        deleted_at: card.deleted_at,
        deleted_by: card.deleted_by,
        delete_batch_id: card.delete_batch_id,
        parent_id: card.parent_id,
        list_id: card.list_id,
        swimlane_id: card.swimlane_id,
        board_id: card.board_id,
        cover_id: card.cover_id,
        color: card.color,
        created_at: card.created_at,
        modified_at: card.modified_at,
        custom_fields: card
            .custom_fields
            .into_iter()
            .map(card_detail_custom_field)
            .collect(),
        date_last_activity: card.date_last_activity,
        description: card.description,
        requested_by: card.requested_by,
        assigned_by: card.assigned_by,
        label_ids: card.label_ids,
        members: card.members,
        assignees: card.assignees,
        requesters: card.requesters,
        assigners: card.assigners,
        received_at: card.received_at,
        start_at: card.start_at,
        due_at: card.due_at,
        end_at: card.end_at,
        due_complete: card.due_complete,
        stickers: card.stickers.into_iter().map(card_detail_sticker).collect(),
        location_name: card.location_name,
        location_address: card.location_address,
        location_latitude: card.location_latitude,
        location_longitude: card.location_longitude,
        locations: card
            .locations
            .into_iter()
            .map(card_detail_location)
            .collect(),
        spent_time: card.spent_time,
        is_overtime: card.is_overtime,
        user_id: card.user_id,
        sort: card.sort,
        subtask_sort: card.subtask_sort,
        card_type: card.card_type,
        linked_id: card.linked_id,
        card_dependencies: card
            .card_dependencies
            .into_iter()
            .map(card_detail_dependency)
            .collect(),
        vote: card.vote.map(card_detail_vote),
        poker: card.poker.map(card_detail_poker),
        target_id_gantt: card.target_id_gantt,
        link_type_gantt: card.link_type_gantt,
        link_id_gantt: card.link_id_gantt,
        card_number: card.card_number,
        show_activities: card.show_activities,
        show_list_on_minicard: card.show_list_on_minicard,
        show_checklist_at_minicard: card.show_checklist_at_minicard,
        hide_finished_checklist_if_items_are_hidden: card
            .hide_finished_checklist_if_items_are_hidden,
    }
}

fn card_detail_custom_field(field: CardCustomField) -> CardDetailCustomField {
    CardDetailCustomField {
        custom_field_id: field.custom_field_id,
        value: field.value.map(card_detail_custom_field_value),
    }
}

fn card_detail_custom_field_value(value: CardCustomFieldValue) -> CardDetailCustomFieldValue {
    match value {
        CardCustomFieldValue::String(value) => CardDetailCustomFieldValue::String(value),
        CardCustomFieldValue::Number(value) => CardDetailCustomFieldValue::Number(value),
        CardCustomFieldValue::Boolean(value) => CardDetailCustomFieldValue::Boolean(value),
        CardCustomFieldValue::Strings(value) => CardDetailCustomFieldValue::Strings(value),
    }
}

fn card_detail_sticker(sticker: CardSticker) -> CardDetailSticker {
    CardDetailSticker {
        icon: sticker.icon,
        name: sticker.name,
        highlight: sticker.highlight.map(card_detail_sticker_highlight),
        position: sticker.position,
    }
}

const fn card_detail_sticker_highlight(
    highlight: CardStickerHighlight,
) -> CardDetailStickerHighlight {
    match highlight {
        CardStickerHighlight::Underline => CardDetailStickerHighlight::Underline,
        CardStickerHighlight::Round => CardDetailStickerHighlight::Round,
    }
}

fn card_detail_location(location: CardLocation) -> CardDetailLocation {
    CardDetailLocation {
        location_id: location.location_id,
        name: location.name,
        address: location.address,
        latitude: location.latitude,
        longitude: location.longitude,
    }
}

fn card_detail_dependency(dependency: CardDependency) -> CardDetailDependency {
    CardDetailDependency {
        card_id: dependency.card_id,
        dependency_type: dependency.dependency_type,
        color: dependency.color,
        icon: dependency.icon,
    }
}

fn card_detail_vote(vote: CardVote) -> CardDetailVote {
    CardDetailVote {
        question: vote.question,
        positive: vote.positive,
        negative: vote.negative,
        end: vote.end,
        public: vote.public,
        allow_non_board_members: vote.allow_non_board_members,
    }
}

fn card_detail_poker(poker: CardPoker) -> CardDetailPoker {
    CardDetailPoker {
        question: poker.question,
        one: poker.one,
        two: poker.two,
        three: poker.three,
        five: poker.five,
        eight: poker.eight,
        thirteen: poker.thirteen,
        twenty: poker.twenty,
        forty: poker.forty,
        one_hundred: poker.one_hundred,
        unsure: poker.unsure,
        end: poker.end,
        allow_non_board_members: poker.allow_non_board_members,
        estimation: poker.estimation,
    }
}
