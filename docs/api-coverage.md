# Wekan API-to-CLI coverage

This tracker inventories every HTTP operation in the corrected Wekan v11.06 contract at [spec/wekan.corrected.yml](../spec/wekan.corrected.yml) and records whether this repository exposes an equivalent first-class CLI command.

User response decoding accepts notifications with absent, null, or populated read
timestamps, including `wekan auth status` and `wekan user current` (`GET /api/user`).
Workspace nodes map `id`, `name`, `icon`, and recursive `children`; unmapped
fields on a node or any child produce `protocol_error` in typed user responses.
Checklist and item detail reads map optional creator `userId` to `user_id`,
including records created by Wekan's UI through DDP. These commands use REST.
Detail operations #15 and #18 preserve denormalized `boardId` as `board_id`,
including the stale source board returned after a REST card move; requests use
the card's current board selector.

**Coverage:** 56 of 143 operations implemented (39.2%); 87 remaining.

## Status rules

- ✓ means a dedicated CLI command currently calls the operation.
- ✗ means no dedicated equivalent CLI command exists yet.
- Coverage counts HTTP operations, not support for every field or action available through an operation.
- Command cells include all command and global options. For these typed commands, `--output` accepts `human` or `json`.
- `--help` (also `-h`) displays command help without calling the API. The root-only `wekan --version` (also `wekan -V`) displays the CLI version.
- Multiple CLI commands may intentionally map to one API operation when Wekan selects behavior through the request body.
- Local-only commands such as `wekan profile ...` are excluded because they do not implement Wekan API operations.
- The generic `wekan api request` escape hatch does not count as dedicated operation coverage; a row becomes implemented only when a first-class typed command owns that operation.

Both list and card updates require at least one supported field flag. List WIP updates accept any subset of `--wip-limit`, `--wip-enabled`, and `--wip-soft`; omitted subfields take Wekan defaults. Card creation permits omitted, empty or explicit-null titles and descriptions. Card update supports same-board movement through `--swimlane-id` and `--list-id`; cross-board movement needs all three `--new-board-id`, `--new-swimlane-id`, and `--new-list-id` values. Set and clear forms of each card field conflict. `--labels` conflicts with both `--label` and `--clear-labels`; `--clear-starred` conflicts with `--starred`. Password stdin modes are mutually exclusive; raw password login conflicts with both code options.

Comment operations #21–23 preserve Wekan's accepted text, including U+0085.
Creation requires `--text` to be nonblank under JavaScript's whitespace rules;
the text is sent unchanged and Wekan trims JavaScript whitespace before storing it.

Checklist commands #13–20 require board/card selectors; item commands additionally
require a checklist selector. Checklist creation permits an omitted title and either
repeated `--item` titles or `--items` JSON (string, string array, or null); those
forms conflict. Item creation permits finite numeric `--sort` or `--sort-json`
(number, string, or null), which conflict. Item update allows no fields, title,
and either boolean `--is-finished` or `--is-finished-json` (boolean or string);
the completion forms conflict. Text and JSON wire types are preserved. Only
checklist and item deletion accept `--yes`.


Custom-field operations #31, #33–40, and #70 use explicit board scope. Definitions
accept seven pinned types; creation defaults settings to `{}` and supports four
boolean display flags, while update also supports `--always-on-card`. Updates
allow no fields. Settings accept explicit null for their four optional properties
and preserve null separately from omission in requests. Repeated dropdown `--item`
conflicts with exact JSON `--items`; card assignment text `--value` conflicts with
`--value-json`. Omission differs
from explicit null. Lookup uses a string path value and no list selector;
assignment requires the list selector. Lookup rejects empty values and the exact
values `.` and `..` before HTTP because the client's URL library cannot preserve
dot-only segments; Wekan itself accepts percent-encoded dots. Target IDs and other
selectors in these commands also reject dot-only segments before HTTP. Identifier
trimming follows Wekan's JavaScript rules, preserving U+0085 and trimming U+FEFF.
Supplied settings reject dropdown IDs that become `.` or `..` after Wekan's JavaScript
whitespace trimming. Only definition and item deletion accept `--yes`. Both
deletion prompts disclose their effect on all
boards associated with a shared definition; the board selector scopes the
lookup rather than limiting deletion to that board. See
[custom-field behavior and results](custom-fields.md).

## Operations

| # | Area | Method | API path | Operation ID | What it does | Equivalent CLI command | Status |
|---:|---|---|---|---|---|---|:---:|
| 1 | Login | `POST` | `/users/login` | `login` | Login with REST API. | `wekan auth login --username <USERNAME> [--code] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth login --username <USERNAME> --password-stdin [--code-stdin] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth login --email <EMAIL> [--code] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth login --email <EMAIL> --password-stdin [--code-stdin] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth login --username <USERNAME> --password-raw-stdin [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth login --email <EMAIL> --password-raw-stdin [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 2 | Login | `POST` | `/users/register` | `register` | Register with REST API. | `wekan auth register --username <USERNAME> [--email <EMAIL>] [--password-stdin | --password-raw-stdin] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth register --email <EMAIL> [--password-stdin | --password-raw-stdin] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 3 | AttachmentStorageSettings | `GET` | `/api/admin/attachment-settings` | `get_attachment_settings` | Get the attachment storage / upload-block settings (GlobalAdmin). | — | ✗ |
| 4 | AttachmentStorageSettings | `PUT` | `/api/admin/attachment-settings` | `update_attachment_settings` | Update the attachment storage / upload-block settings (GlobalAdmin). | — | ✗ |
| 5 | Users | `GET` | `/api/admin/domains` | `get_admin_domains` | List email domains with their user counts (GlobalAdmin). | — | ✗ |
| 6 | Org | `GET` | `/api/admin/orgs` | `get_admin_orgs` | List organizations with their feature toggles (GlobalAdmin). | — | ✗ |
| 7 | Org | `PUT` | `/api/admin/orgs/{org}/features` | `update_admin_org_features` | Set feature toggles on one organization (GlobalAdmin). | — | ✗ |
| 8 | Org | `PUT` | `/api/admin/orgs/features` | `update_all_admin_org_features` | Set one feature toggle on all organizations (GlobalAdmin). | — | ✗ |
| 9 | Team | `GET` | `/api/admin/teams` | `get_admin_teams` | List teams with their feature toggles (GlobalAdmin). | — | ✗ |
| 10 | Team | `PUT` | `/api/admin/teams/{team}/features` | `update_admin_team_features` | Set feature toggles on one team (GlobalAdmin). | — | ✗ |
| 11 | Team | `PUT` | `/api/admin/teams/features` | `update_all_admin_team_features` | Set one feature toggle on all teams (GlobalAdmin). | — | ✗ |
| 12 | Boards | `GET` | `/api/boards/{board}/attachments/{attachment}/export` | `exportAttachmentJson` | Export an attachment as JSON. | — | ✗ |
| 13 | Checklists | `GET` | `/api/boards/{board}/cards/{card}/checklists` | `get_board_card_checklists` | List the checklists on a card. | `wekan checklist list --board <BOARD_ID> --card <CARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 14 | Checklists | `POST` | `/api/boards/{board}/cards/{card}/checklists` | `post_board_card_checklists` | Create a checklist on a card. | `wekan checklist create --board <BOARD_ID> --card <CARD_ID> [--title <TEXT>] [--item <TEXT>]... [--items <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 15 | Checklists | `GET` | `/api/boards/{board}/cards/{card}/checklists/{checklist}` | `get_board_card_checklist` | Get one checklist on a card. | `wekan checklist get <CHECKLIST_ID> --board <BOARD_ID> --card <CARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 16 | Checklists | `DELETE` | `/api/boards/{board}/cards/{card}/checklists/{checklist}` | `delete_board_card_checklist` | Delete a checklist from a card. | `wekan checklist delete <CHECKLIST_ID> --board <BOARD_ID> --card <CARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 17 | ChecklistItems | `POST` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items` | `post_board_card_checklist_items` | Add an item to a card checklist. | `wekan checklist-item create --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> --title <TEXT> [--sort <NUMBER>] [--sort-json <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 18 | ChecklistItems | `GET` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `get_board_card_checklist_item` | Get one checklist item. | `wekan checklist-item get <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 19 | ChecklistItems | `PUT` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `put_board_card_checklist_item` | Update a checklist item. | `wekan checklist-item update <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> [--title <TEXT>] [--is-finished <BOOL>] [--is-finished-json <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 20 | ChecklistItems | `DELETE` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `delete_board_card_checklist_item` | Delete a checklist item. | `wekan checklist-item delete <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 21 | CardComments | `GET` | `/api/boards/{board}/cards/{card}/comments` | `get_board_card_comments` | List comments on a card. | `wekan comment list --board <BOARD_ID> --card <CARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 22 | CardComments | `POST` | `/api/boards/{board}/cards/{card}/comments` | `post_board_card_comments` | Create a comment on a card. | `wekan comment create --board <BOARD_ID> --card <CARD_ID> --text <TEXT> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 23 | CardComments | `GET` | `/api/boards/{board}/cards/{card}/comments/{comment}` | `get_board_card_comment` | Get one card comment. | `wekan comment get <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 24 | CardComments | `DELETE` | `/api/boards/{board}/cards/{card}/comments/{comment}` | `delete_board_card_comment` | Delete a card comment. | `wekan comment delete <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 25 | Dependencies | `GET` | `/api/boards/{board}/cards/{card}/dependencies` | `get_card_dependencies` | Get one card's dependencies. | — | ✗ |
| 26 | Dependencies | `POST` | `/api/boards/{board}/cards/{card}/dependencies` | `new_card_dependency` | Add (or update) a typed dependency line from a card to another card. | — | ✗ |
| 27 | Dependencies | `PUT` | `/api/boards/{board}/cards/{card}/dependencies/{target}` | `edit_card_dependency` | Edit a dependency line's type, color or icon. | — | ✗ |
| 28 | Dependencies | `DELETE` | `/api/boards/{board}/cards/{card}/dependencies/{target}` | `delete_card_dependency` | Remove a dependency line from a card. | — | ✗ |
| 29 | Cards | `DELETE` | `/api/boards/{board}/cards/bulk` | `delete_board_cards_bulk` | Delete multiple cards from a board. | — | ✗ |
| 30 | Cards | `POST` | `/api/boards/{board}/cards/labels` | `post_board_cards_labels` | Apply labels to multiple cards on a board. | — | ✗ |
| 31 | Cards | `GET` | `/api/boards/{board}/cardsByCustomField/{customField}/{customFieldValue}` | `get_board_customFieldValue` | List cards whose custom field has a specified value. | `wekan card find-by-custom-field --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> --value <TEXT> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 32 | Cards | `GET` | `/api/boards/{board}/cards_count` | `get_board_cards_count` | Count cards on a board. | — | ✗ |
| 33 | CustomFields | `GET` | `/api/boards/{board}/custom-fields` | `get_board_custom-fields` | List custom-field definitions on a board. | `wekan custom-field list --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 34 | CustomFields | `POST` | `/api/boards/{board}/custom-fields` | `post_board_custom-fields` | Create a custom-field definition on a board. | `wekan custom-field create --board <BOARD_ID> --name <NAME> --type <TYPE> [--settings <JSON>] [--show-on-card <BOOL>] [--automatically-on-card <BOOL>] [--show-label-on-mini-card <BOOL>] [--show-sum-at-top-of-list <BOOL>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 35 | CustomFields | `GET` | `/api/boards/{board}/custom-fields/{customField}` | `get_board_customField` | Get one custom-field definition. | `wekan custom-field get <CUSTOM_FIELD_ID> --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 36 | CustomFields | `PUT` | `/api/boards/{board}/custom-fields/{customField}` | `put_board_customField` | Update a custom-field definition. | `wekan custom-field update <CUSTOM_FIELD_ID> --board <BOARD_ID> [--name <NAME>] [--type <TYPE>] [--settings <JSON>] [--show-on-card <BOOL>] [--automatically-on-card <BOOL>] [--show-label-on-mini-card <BOOL>] [--show-sum-at-top-of-list <BOOL>] [--always-on-card <BOOL>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 37 | CustomFields | `DELETE` | `/api/boards/{board}/custom-fields/{customField}` | `delete_board_customField` | Delete a custom-field definition. | `wekan custom-field delete <CUSTOM_FIELD_ID> --board <BOARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 38 | CustomFields | `POST` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items` | `post_board_customField_dropdown-items` | Add a dropdown option to a custom field. | `wekan custom-field-item create --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--item <TEXT>]... [--items <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 39 | CustomFields | `PUT` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items/{dropdownItem}` | `put_board_customField_dropdownItem` | Update a custom-field dropdown option. | `wekan custom-field-item update <ITEM_ID> --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--name <NAME>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 40 | CustomFields | `DELETE` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items/{dropdownItem}` | `delete_board_customField_dropdownItem` | Delete a custom-field dropdown option. | `wekan custom-field-item delete <ITEM_ID> --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 41 | Dependencies | `GET` | `/api/boards/{board}/dependencies` | `get_board_dependencies` | Get all card dependency lines ("Red Strings") of a board. | — | ✗ |
| 42 | Boards | `GET` | `/api/boards/{board}/export` | `exportJson` | This route is used to export the board to a json file format. | — | ✗ |
| 43 | Boards | `GET` | `/api/boards/{board}/export/{format}` | `exportExternal` | Export the board as a NextCloud Deck / OpenProject / GitHub /  GitLab / Gitea / Forgejo style JSON. | — | ✗ |
| 44 | Boards | `GET` | `/api/boards/{board}/export/csv` | `exportCSVTSV` | This route is used to export the board to a CSV or TSV file format. | — | ✗ |
| 45 | Boards | `GET` | `/api/boards/{board}/export/kanboard` | `exportKanboard` | Export the board as a Kanboard-style JSON (columns + tasks). | — | ✗ |
| 46 | Boards | `GET` | `/api/boards/{board}/exportExcel` | `exportExcel` | This route is used to export the board Excel. | — | ✗ |
| 47 | Boards | `GET` | `/api/boards/{board}/exportPDF` | `exportBoardPDF` | Export a whole board to PDF (board title, lists and their cards). | — | ✗ |
| 48 | Boards | `GET` | `/api/boards/{board}/exportZip` | `exportZip` | Export as a .zip - the JSON document, and the attachments as files. | — | ✗ |
| 49 | Integrations | `GET` | `/api/boards/{board}/integrations` | `get_board_integrations` | List integrations configured for a board. | — | ✗ |
| 50 | Integrations | `POST` | `/api/boards/{board}/integrations` | `post_board_integrations` | Create a board integration. | — | ✗ |
| 51 | Integrations | `GET` | `/api/boards/{board}/integrations/{int}` | `get_board_int` | Get one board integration. | — | ✗ |
| 52 | Integrations | `PUT` | `/api/boards/{board}/integrations/{int}` | `put_board_int` | Update a board integration. | — | ✗ |
| 53 | Integrations | `DELETE` | `/api/boards/{board}/integrations/{int}` | `delete_board_int` | Delete a board integration. | — | ✗ |
| 54 | Integrations | `DELETE` | `/api/boards/{board}/integrations/{int}/activities` | `delete_board_int_activities` | Delete activities recorded for a board integration. | — | ✗ |
| 55 | Integrations | `POST` | `/api/boards/{board}/integrations/{int}/activities` | `post_board_int_activities` | Create an activity for a board integration. | — | ✗ |
| 56 | Lists | `GET` | `/api/boards/{board}/lists` | `get_board_lists` | List the lists on a board. | `wekan list list --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 57 | Lists | `POST` | `/api/boards/{board}/lists` | `post_board_lists` | Create a list on a board. | `wekan list create --board <BOARD_ID> --title <TITLE> [--swimlane-id <ID>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 58 | Lists | `GET` | `/api/boards/{board}/lists/{list}` | `get_board_list` | Get one list on a board. | `wekan list get <LIST_ID> --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 59 | Lists | `PUT` | `/api/boards/{board}/lists/{list}` | `put_board_list` | Update a list on a board. | `wekan list update <LIST_ID> --board <BOARD_ID> [--title <TITLE>] [--color <COLOR>] [--starred <BOOL> | --clear-starred] [--wip-limit <NUMBER>] [--wip-enabled <BOOL>] [--wip-soft <BOOL>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 60 | Lists | `DELETE` | `/api/boards/{board}/lists/{list}` | `delete_board_list` | Delete a list from a board. | `wekan list delete <LIST_ID> --board <BOARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 61 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards` | `get_board_list_cards` | List the cards in a board list. | `wekan card list --board <BOARD_ID> --list <LIST_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 62 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards` | `post_board_list_cards` | Create a card in a board list. | `wekan card create --board <BOARD_ID> --list <LIST_ID> --swimlane-id <ID> [--title <TITLE> | --null-title] [--linked-id <CARD_ID>] [--description <TEXT> | --null-description] [--member <USER_ID>]... [--assignee <USER_ID>]... [--received-at <DATE>] [--start-at <DATE>] [--due-at <DATE>] [--end-at <DATE>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 63 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}` | `get_board_list_card` | Get one card from a board list. | `wekan card get <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 64 | Cards | `PUT` | `/api/boards/{board}/lists/{list}/cards/{card}` | `put_board_list_card` | Update or move a card. | `wekan card update <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--title <TITLE>] [--sort <NUMBER>] [--parent-id <CARD_ID>] [--description <TEXT>] [--color <COLOR>] [--label <LABEL_ID>]... [--labels <JSON>] [--clear-labels] [--requested-by <NAME>] [--assigned-by <NAME>] [--received-at <DATE>] [--clear-received-at] [--start-at <DATE>] [--clear-start-at] [--due-at <DATE>] [--clear-due-at] [--end-at <DATE>] [--clear-end-at] [--spent-time <NUMBER>] [--is-over-time <BOOL>] [--member <USER_ID>]... [--clear-members] [--assignee <USER_ID>]... [--clear-assignees] [--due-complete <BOOL>] [--swimlane-id <ID>] [--list-id <LIST_ID>] [--new-board-id <BOARD_ID>] [--new-swimlane-id <ID>] [--new-list-id <LIST_ID>] [--archive <BOOL>] [--vote <JSON>] [--poker <JSON>] [--custom-fields <JSON>] [--stickers <JSON>] [--locations <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 65 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}` | `delete_board_list_card` | Delete a card from a board list. | `wekan card delete <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 66 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/archive` | `post_board_list_card_archive` | Archive an active card and its child cards. | `wekan card archive <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 67 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/assignees/{assignee}` | `post_board_list_card_assignee` | Add an assignee to a card. | — | ✗ |
| 68 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}/assignees/{assignee}` | `delete_board_list_card_assignee` | Remove an assignee from a card. | — | ✗ |
| 69 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/copy` | `post_board_list_card_copy` | Copy a card. | — | ✗ |
| 70 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/customFields/{customField}` | `post_board_list_card_customField` | Set a custom-field value on a card. | `wekan card set-custom-field <CARD_ID> --board <BOARD_ID> --list <LIST_ID> --custom-field <CUSTOM_FIELD_ID> [--value <TEXT>] [--value-json <JSON>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 71 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}/exportExcel` | `exportExcelCard` | Export a single card to Excel (.xlsx), formatted for DIN A4 Portrait printing. | — | ✗ |
| 72 | Boards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}/exportPDF` | `exportCardPDF` | Export a single card to PDF. | — | ✗ |
| 73 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/members/{member}` | `post_board_list_card_member` | Add a member to a card. | — | ✗ |
| 74 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}/members/{member}` | `delete_board_list_card_member` | Remove a member from a card. | — | ✗ |
| 75 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/unarchive` | `post_board_list_card_unarchive` | Restore an archived card and its child cards. | `wekan card restore <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 76 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/bulk` | `post_board_list_cards_bulk` | Create multiple cards in a board list. | — | ✗ |
| 77 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards_count` | `get_board_list_cards_count` | Count cards in a board list. | — | ✗ |
| 78 | Lists | `POST` | `/api/boards/{board}/lists/{list}/copy` | `post_board_list_copy` | Copy a board list. | — | ✗ |
| 79 | Lists | `POST` | `/api/boards/{board}/lists/{list}/move` | `post_board_list_move` | Move a board list. | — | ✗ |
| 80 | Users | `POST` | `/api/boards/{board}/members/{user}/add` | `post_board_user_add` | Add a user as a board member. | — | ✗ |
| 81 | Users | `POST` | `/api/boards/{board}/members/{user}/remove` | `post_board_user_remove` | Remove a user from a board. | — | ✗ |
| 82 | Rules | `GET` | `/api/boards/{board}/rules` | `get_board_rules` | Get the list of automation rules of a board. | — | ✗ |
| 83 | Rules | `POST` | `/api/boards/{board}/rules` | `new_board_rule` | Add an automation rule to a board. | — | ✗ |
| 84 | Rules | `GET` | `/api/boards/{board}/rules/{rule}` | `get_board_rule` | Get a single automation rule. | — | ✗ |
| 85 | Rules | `PUT` | `/api/boards/{board}/rules/{rule}` | `edit_board_rule` | Edit an automation rule. | — | ✗ |
| 86 | Rules | `DELETE` | `/api/boards/{board}/rules/{rule}` | `delete_board_rule` | Remove an automation rule (and its trigger and action). | — | ✗ |
| 87 | Swimlanes | `GET` | `/api/boards/{board}/swimlanes` | `get_board_swimlanes` | List the swimlanes on a board. | `wekan swimlane list --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 88 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes` | `post_board_swimlanes` | Create a swimlane on a board. | `wekan swimlane create --board <BOARD_ID> --title <TITLE> [--sort <NUMBER>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 89 | Swimlanes | `GET` | `/api/boards/{board}/swimlanes/{swimlane}` | `get_board_swimlane` | Get one swimlane on a board. | `wekan swimlane get <SWIMLANE_ID> --board <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 90 | Swimlanes | `PUT` | `/api/boards/{board}/swimlanes/{swimlane}` | `put_board_swimlane` | Update a swimlane on a board. | `wekan swimlane update <SWIMLANE_ID> --board <BOARD_ID> --title <TITLE> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 91 | Swimlanes | `DELETE` | `/api/boards/{board}/swimlanes/{swimlane}` | `delete_board_swimlane` | Delete a swimlane from a board. | `wekan swimlane delete <SWIMLANE_ID> --board <BOARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 92 | Cards | `GET` | `/api/boards/{board}/swimlanes/{swimlane}/cards` | `get_board_swimlane_cards` | List cards in a board swimlane. | — | ✗ |
| 93 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes/{swimlane}/copy` | `post_board_swimlane_copy` | Copy a board swimlane. | — | ✗ |
| 94 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes/{swimlane}/move` | `post_board_swimlane_move` | Move a board swimlane. | — | ✗ |
| 95 | Cards | `GET` | `/api/cards/{card}` | `get_card` | Get a card by its ID. | — | ✗ |
| 96 | Users | `POST` | `/api/createtoken/{user}` | `post_user` | Create an API token for a user. | — | ✗ |
| 97 | Users | `POST` | `/api/deletetoken` | `post_deletetoken` | Delete an API token. | — | ✗ |
| 98 | Boards | `POST` | `/api/import/zip` | `importZip` | Import a .zip export - the document and its attachment files. | — | ✗ |
| 99 | Settings | `GET` | `/api/settings` | `get_global_settings` | Get the global Admin Panel settings. | — | ✗ |
| 100 | Settings | `PUT` | `/api/settings` | `update_global_settings` | Update the global Admin Panel settings. | — | ✗ |
| 101 | Users | `GET` | `/api/user` | `get_current_user` | Return the currently authenticated user. | `wekan user current [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth status [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 102 | Cards | `GET` | `/api/user/cards` | `get_user_cards` | List readable cards related to the current user. | `wekan user cards [--due] [--from <DATE>] [--to <DATE>] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 103 | Users | `GET` | `/api/users` | `get_users` | List users (site admin only). | `wekan user list [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 104 | Users | `POST` | `/api/users` | `post_users` | Create a user (site admin only). | `wekan user create --username <USERNAME> --email <EMAIL> [--password-stdin | --password-raw-stdin | --password-digest-stdin] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 105 | Users | `GET` | `/api/users/{user}` | `get_user` | Return a user by ID or username (site admin only). | `wekan user get <USER> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 106 | Users | `PUT` | `/api/users/{user}` | `put_user` | Perform an administrative action on a user. | `wekan user take-ownership <USER_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan user disable-login <USER_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan user enable-login <USER_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan user update <USER_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 107 | Users | `DELETE` | `/api/users/{user}` | `delete_user` | Delete a user (site admin only). | `wekan user delete <USER_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 108 | Attachments | `POST` | `/api/attachment/upload` | `upload_attachment` | Upload a file as a card attachment. | — | ✗ |
| 109 | Attachments | `POST` | `/api/attachment/upload-background` | `upload_board_background` | Upload a board background image. | — | ✗ |
| 110 | Attachments | `GET` | `/api/attachment/download-background/{boardId}` | `download_board_background` | Download the board's current background image as base64. | — | ✗ |
| 111 | Attachments | `GET` | `/api/attachment/download/{attachmentId}` | `download_attachment` | Download an attachment as base64. | — | ✗ |
| 112 | Attachments | `GET` | `/api/attachment/info/{attachmentId}` | `get_attachment_info` | Get attachment metadata. | — | ✗ |
| 113 | Attachments | `GET` | `/api/attachment/list/{boardId}` | `list_board_attachments` | List all attachments of a board. | — | ✗ |
| 114 | Attachments | `GET` | `/api/attachment/list/{boardId}/{swimlaneId}` | `list_swimlane_attachments` | List attachments of a board filtered by swimlane. | — | ✗ |
| 115 | Attachments | `GET` | `/api/attachment/list/{boardId}/{swimlaneId}/{listId}` | `list_list_attachments` | List attachments of a board filtered by swimlane and list. | — | ✗ |
| 116 | Attachments | `GET` | `/api/attachment/list/{boardId}/{swimlaneId}/{listId}/{cardId}` | `list_card_attachments` | List attachments of a single card. | — | ✗ |
| 117 | Attachments | `POST` | `/api/attachment/copy` | `copy_attachment` | Copy an attachment to another card. | — | ✗ |
| 118 | Attachments | `POST` | `/api/attachment/move` | `move_attachment` | Move an attachment to another card. | — | ✗ |
| 119 | Attachments | `DELETE` | `/api/attachment/delete/{attachmentId}` | `delete_attachment` | Delete an attachment. | — | ✗ |
| 120 | Boards | `GET` | `/api/boards/{boardId}/cardSettings` | `get_board_card_settings` | Get a board's card settings (display toggles + card aging). | — | ✗ |
| 121 | Boards | `PUT` | `/api/boards/{boardId}/cardSettings` | `edit_board_card_settings` | Update a board's card settings (display toggles + card aging). | — | ✗ |
| 122 | Cards | `POST` | `/api/boards/{boardId}/swimlanes/{swimlaneId}/lists/{listId}/ics` | `import_ics` | Import an iCalendar (.ics) file into a board as cards. | — | ✗ |
| 123 | Authentication | `POST` | `/users/logout` | `logout` | Revoke the current login token or all login tokens. | `wekan auth logout [--all] [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan auth logout --local-only [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` (the `--local-only` form performs local credential cleanup without calling this operation) | ✓ |
| 124 | Users | `GET` | `/api/users/{userId}/boards` | `get_user_boards` | List a user's active boards (self or site admin). | `wekan user boards <USER_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`wekan board list [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]`<br>`--public` selects operation #125 instead. | ✓ |
| 125 | Boards | `GET` | `/api/boards` | `get_public_boards` | List public boards. | `wekan board list --public [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 126 | Boards | `POST` | `/api/boards` | `create_board` | Create a board and its default swimlane. | `wekan board create --title <TITLE> [--owner <USER_ID>] [--is-admin <BOOL>] [--is-active <BOOL>] [--permission <PERMISSION>] [--color <COLOR>] [--no-comments] [--comment-only] [--worker] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 127 | Boards | `GET` | `/api/boards_count` | `get_boards_count` | Count private and public boards. | `wekan board count [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 128 | Boards | `GET` | `/api/boards/{boardId}` | `get_board` | Get a board. | `wekan board get <BOARD_ID> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 129 | Boards | `DELETE` | `/api/boards/{boardId}` | `delete_board` | Delete a board. | `wekan board delete <BOARD_ID> [--yes] [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 130 | Boards | `POST` | `/api/boards/import` | `import_board` | Import a Wekan board export. | — | ✗ |
| 131 | Boards | `POST` | `/api/boards/import/{source}` | `import_board_from` | Import a board exported by a supported external tool. | — | ✗ |
| 132 | Boards | `PUT` | `/api/boards/{boardId}/title` | `update_board_title` | Update a board title. | `wekan board rename <BOARD_ID> --title <TITLE> [--server <URL>] [--profile <PROFILE>] [--output <FORMAT>] [--allow-insecure-http] [--help]` | ✓ |
| 133 | Boards | `PUT` | `/api/boards/{boardId}/labels` | `add_board_label` | Add a board label if it does not already exist. | — | ✗ |
| 134 | Boards | `POST` | `/api/boards/{boardId}/copy` | `copy_board` | Copy a board. | — | ✗ |
| 135 | Boards | `POST` | `/api/boards/{boardId}/members/{memberId}` | `update_board_member_permissions` | Set a board member's role or individual permission flags. | — | ✗ |
| 136 | Boards | `GET` | `/api/boards/{boardId}/domains` | `get_board_domains` | List email domains a board is shared with. | — | ✗ |
| 137 | Boards | `POST` | `/api/boards/{boardId}/domains` | `add_board_domain` | Share a board with an email domain. | — | ✗ |
| 138 | Boards | `DELETE` | `/api/boards/{boardId}/domains/{domain}` | `delete_board_domain` | Stop sharing a board with an email domain. | — | ✗ |
| 139 | Boards | `GET` | `/api/boards/{boardId}/attachments` | `get_board_attachments` | List board attachment links using the legacy response shape. | — | ✗ |
| 140 | System | `GET` | `/schema-upgrade-status` | `get_schema_upgrade_status` | Get live startup schema-upgrade status. | — | ✗ |
| 141 | Authentication | `OPTIONS` | `/users/login` | `options_login` | Authentication preflight with an empty response. | — | ✗ |
| 142 | Authentication | `OPTIONS` | `/users/register` | `options_register` | Authentication preflight with an empty response. | — | ✗ |
| 143 | Authentication | `OPTIONS` | `/users/logout` | `options_logout` | Authentication preflight with an empty response. | — | ✗ |
