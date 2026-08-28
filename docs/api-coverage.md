# Wekan API-to-CLI coverage

This tracker inventories every HTTP operation in the corrected Wekan v11.06 contract at [spec/wekan.corrected.yml](../spec/wekan.corrected.yml) and records whether this repository exposes an equivalent first-class CLI command.

**Coverage:** 17 of 140 operations implemented (12.1%); 123 remaining.

## Status rules

- ✓ means a dedicated CLI command currently calls the operation.
- ✗ means no dedicated equivalent CLI command exists yet.
- Multiple CLI commands may intentionally map to one API operation when Wekan selects behavior through the request body.
- Local-only commands such as `wekan profile ...` are excluded because they do not implement Wekan API operations.

## Operations

| # | Area | Method | API path | Operation ID | What it does | Equivalent CLI command | Status |
|---:|---|---|---|---|---|---|:---:|
| 1 | Login | `POST` | `/users/login` | `login` | Login with REST API. | `wekan auth login` | ✓ |
| 2 | Login | `POST` | `/users/register` | `register` | Register with REST API. | `wekan auth register` | ✓ |
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
| 13 | Checklists | `GET` | `/api/boards/{board}/cards/{card}/checklists` | `get_board_card_checklists` | List the checklists on a card. | — | ✗ |
| 14 | Checklists | `POST` | `/api/boards/{board}/cards/{card}/checklists` | `post_board_card_checklists` | Create a checklist on a card. | — | ✗ |
| 15 | Checklists | `GET` | `/api/boards/{board}/cards/{card}/checklists/{checklist}` | `get_board_card_checklist` | Get one checklist on a card. | — | ✗ |
| 16 | Checklists | `DELETE` | `/api/boards/{board}/cards/{card}/checklists/{checklist}` | `delete_board_card_checklist` | Delete a checklist from a card. | — | ✗ |
| 17 | ChecklistItems | `POST` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items` | `post_board_card_checklist_items` | Add an item to a card checklist. | — | ✗ |
| 18 | ChecklistItems | `GET` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `get_board_card_checklist_item` | Get one checklist item. | — | ✗ |
| 19 | ChecklistItems | `PUT` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `put_board_card_checklist_item` | Update a checklist item. | — | ✗ |
| 20 | ChecklistItems | `DELETE` | `/api/boards/{board}/cards/{card}/checklists/{checklist}/items/{item}` | `delete_board_card_checklist_item` | Delete a checklist item. | — | ✗ |
| 21 | CardComments | `GET` | `/api/boards/{board}/cards/{card}/comments` | `get_board_card_comments` | List comments on a card. | — | ✗ |
| 22 | CardComments | `POST` | `/api/boards/{board}/cards/{card}/comments` | `post_board_card_comments` | Create a comment on a card. | — | ✗ |
| 23 | CardComments | `GET` | `/api/boards/{board}/cards/{card}/comments/{comment}` | `get_board_card_comment` | Get one card comment. | — | ✗ |
| 24 | CardComments | `DELETE` | `/api/boards/{board}/cards/{card}/comments/{comment}` | `delete_board_card_comment` | Delete a card comment. | — | ✗ |
| 25 | Dependencies | `GET` | `/api/boards/{board}/cards/{card}/dependencies` | `get_card_dependencies` | Get one card's dependencies. | — | ✗ |
| 26 | Dependencies | `POST` | `/api/boards/{board}/cards/{card}/dependencies` | `new_card_dependency` | Add (or update) a typed dependency line from a card to another card. | — | ✗ |
| 27 | Dependencies | `PUT` | `/api/boards/{board}/cards/{card}/dependencies/{target}` | `edit_card_dependency` | Edit a dependency line's type, color or icon. | — | ✗ |
| 28 | Dependencies | `DELETE` | `/api/boards/{board}/cards/{card}/dependencies/{target}` | `delete_card_dependency` | Remove a dependency line from a card. | — | ✗ |
| 29 | Cards | `DELETE` | `/api/boards/{board}/cards/bulk` | `delete_board_cards_bulk` | Delete multiple cards from a board. | — | ✗ |
| 30 | Cards | `POST` | `/api/boards/{board}/cards/labels` | `post_board_cards_labels` | Apply labels to multiple cards on a board. | — | ✗ |
| 31 | Cards | `GET` | `/api/boards/{board}/cardsByCustomField/{customField}/{customFieldValue}` | `get_board_customFieldValue` | List cards whose custom field has a specified value. | — | ✗ |
| 32 | Cards | `GET` | `/api/boards/{board}/cards_count` | `get_board_cards_count` | Count cards on a board. | — | ✗ |
| 33 | CustomFields | `GET` | `/api/boards/{board}/custom-fields` | `get_board_custom-fields` | List custom-field definitions on a board. | — | ✗ |
| 34 | CustomFields | `POST` | `/api/boards/{board}/custom-fields` | `post_board_custom-fields` | Create a custom-field definition on a board. | — | ✗ |
| 35 | CustomFields | `GET` | `/api/boards/{board}/custom-fields/{customField}` | `get_board_customField` | Get one custom-field definition. | — | ✗ |
| 36 | CustomFields | `PUT` | `/api/boards/{board}/custom-fields/{customField}` | `put_board_customField` | Update a custom-field definition. | — | ✗ |
| 37 | CustomFields | `DELETE` | `/api/boards/{board}/custom-fields/{customField}` | `delete_board_customField` | Delete a custom-field definition. | — | ✗ |
| 38 | CustomFields | `POST` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items` | `post_board_customField_dropdown-items` | Add a dropdown option to a custom field. | — | ✗ |
| 39 | CustomFields | `PUT` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items/{dropdownItem}` | `put_board_customField_dropdownItem` | Update a custom-field dropdown option. | — | ✗ |
| 40 | CustomFields | `DELETE` | `/api/boards/{board}/custom-fields/{customField}/dropdown-items/{dropdownItem}` | `delete_board_customField_dropdownItem` | Delete a custom-field dropdown option. | — | ✗ |
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
| 56 | Lists | `GET` | `/api/boards/{board}/lists` | `get_board_lists` | List the lists on a board. | — | ✗ |
| 57 | Lists | `POST` | `/api/boards/{board}/lists` | `post_board_lists` | Create a list on a board. | — | ✗ |
| 58 | Lists | `GET` | `/api/boards/{board}/lists/{list}` | `get_board_list` | Get one list on a board. | — | ✗ |
| 59 | Lists | `PUT` | `/api/boards/{board}/lists/{list}` | `put_board_list` | Update a list on a board. | — | ✗ |
| 60 | Lists | `DELETE` | `/api/boards/{board}/lists/{list}` | `delete_board_list` | Delete a list from a board. | — | ✗ |
| 61 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards` | `get_board_list_cards` | List the cards in a board list. | — | ✗ |
| 62 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards` | `post_board_list_cards` | Create a card in a board list. | — | ✗ |
| 63 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}` | `get_board_list_card` | Get one card from a board list. | — | ✗ |
| 64 | Cards | `PUT` | `/api/boards/{board}/lists/{list}/cards/{card}` | `put_board_list_card` | Update or move a card. | — | ✗ |
| 65 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}` | `delete_board_list_card` | Delete a card from a board list. | — | ✗ |
| 66 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/archive` | `post_board_list_card_archive` | Archive a card. | — | ✗ |
| 67 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/assignees/{assignee}` | `post_board_list_card_assignee` | Add an assignee to a card. | — | ✗ |
| 68 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}/assignees/{assignee}` | `delete_board_list_card_assignee` | Remove an assignee from a card. | — | ✗ |
| 69 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/copy` | `post_board_list_card_copy` | Copy a card. | — | ✗ |
| 70 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/customFields/{customField}` | `post_board_list_card_customField` | Set a custom-field value on a card. | — | ✗ |
| 71 | Cards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}/exportExcel` | `exportExcelCard` | Export a single card to Excel (.xlsx), formatted for DIN A4 Portrait printing. | — | ✗ |
| 72 | Boards | `GET` | `/api/boards/{board}/lists/{list}/cards/{card}/exportPDF` | `exportCardPDF` | Export a single card to PDF. | — | ✗ |
| 73 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/members/{member}` | `post_board_list_card_member` | Add a member to a card. | — | ✗ |
| 74 | Cards | `DELETE` | `/api/boards/{board}/lists/{list}/cards/{card}/members/{member}` | `delete_board_list_card_member` | Remove a member from a card. | — | ✗ |
| 75 | Cards | `POST` | `/api/boards/{board}/lists/{list}/cards/{card}/unarchive` | `post_board_list_card_unarchive` | Restore an archived card. | — | ✗ |
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
| 87 | Swimlanes | `GET` | `/api/boards/{board}/swimlanes` | `get_board_swimlanes` | List the swimlanes on a board. | — | ✗ |
| 88 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes` | `post_board_swimlanes` | Create a swimlane on a board. | — | ✗ |
| 89 | Swimlanes | `GET` | `/api/boards/{board}/swimlanes/{swimlane}` | `get_board_swimlane` | Get one swimlane on a board. | — | ✗ |
| 90 | Swimlanes | `PUT` | `/api/boards/{board}/swimlanes/{swimlane}` | `put_board_swimlane` | Update a swimlane on a board. | — | ✗ |
| 91 | Swimlanes | `DELETE` | `/api/boards/{board}/swimlanes/{swimlane}` | `delete_board_swimlane` | Delete a swimlane from a board. | — | ✗ |
| 92 | Cards | `GET` | `/api/boards/{board}/swimlanes/{swimlane}/cards` | `get_board_swimlane_cards` | List cards in a board swimlane. | — | ✗ |
| 93 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes/{swimlane}/copy` | `post_board_swimlane_copy` | Copy a board swimlane. | — | ✗ |
| 94 | Swimlanes | `POST` | `/api/boards/{board}/swimlanes/{swimlane}/move` | `post_board_swimlane_move` | Move a board swimlane. | — | ✗ |
| 95 | Cards | `GET` | `/api/cards/{card}` | `get_card` | Get a card by its ID. | — | ✗ |
| 96 | Users | `POST` | `/api/createtoken/{user}` | `post_user` | Create an API token for a user. | — | ✗ |
| 97 | Users | `POST` | `/api/deletetoken` | `post_deletetoken` | Delete an API token. | — | ✗ |
| 98 | Boards | `POST` | `/api/import/zip` | `importZip` | Import a .zip export - the document and its attachment files. | — | ✗ |
| 99 | Settings | `GET` | `/api/settings` | `get_global_settings` | Get the global Admin Panel settings. | — | ✗ |
| 100 | Settings | `PUT` | `/api/settings` | `update_global_settings` | Update the global Admin Panel settings. | — | ✗ |
| 101 | Users | `GET` | `/api/user` | `get_current_user` | Return the currently authenticated user. | `wekan user current`<br>`wekan auth status` | ✓ |
| 102 | Cards | `GET` | `/api/user/cards` | `get_user_cards` | List readable cards related to the current user. | `wekan user cards` | ✓ |
| 103 | Users | `GET` | `/api/users` | `get_users` | List users (site admin only). | `wekan user list` | ✓ |
| 104 | Users | `POST` | `/api/users` | `post_users` | Create a user (site admin only). | `wekan user create` | ✓ |
| 105 | Users | `GET` | `/api/users/{user}` | `get_user` | Return a user by ID or username (site admin only). | `wekan user get` | ✓ |
| 106 | Users | `PUT` | `/api/users/{user}` | `put_user` | Perform an administrative action on a user. | `wekan user take-ownership`<br>`wekan user disable-login`<br>`wekan user enable-login` | ✓ |
| 107 | Users | `DELETE` | `/api/users/{user}` | `delete_user` | Delete a user (site admin only). | `wekan user delete` | ✓ |
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
| 123 | Authentication | `POST` | `/users/logout` | `logout` | Revoke the current login token or all login tokens. | `wekan auth logout` | ✓ |
| 124 | Users | `GET` | `/api/users/{userId}/boards` | `get_user_boards` | List a user's active boards (self or site admin). | `wekan user boards`<br>`wekan board list` | ✓ |
| 125 | Boards | `GET` | `/api/boards` | `get_public_boards` | List public boards. | `wekan board list --public` | ✓ |
| 126 | Boards | `POST` | `/api/boards` | `create_board` | Create a board and its default swimlane. | `wekan board create` | ✓ |
| 127 | Boards | `GET` | `/api/boards_count` | `get_boards_count` | Count private and public boards. | `wekan board count` | ✓ |
| 128 | Boards | `GET` | `/api/boards/{boardId}` | `get_board` | Get a board. | `wekan board get` | ✓ |
| 129 | Boards | `DELETE` | `/api/boards/{boardId}` | `delete_board` | Delete a board. | `wekan board delete` | ✓ |
| 130 | Boards | `POST` | `/api/boards/import` | `import_board` | Import a Wekan board export. | — | ✗ |
| 131 | Boards | `POST` | `/api/boards/import/{source}` | `import_board_from` | Import a board exported by a supported external tool. | — | ✗ |
| 132 | Boards | `PUT` | `/api/boards/{boardId}/title` | `update_board_title` | Update a board title. | `wekan board rename` | ✓ |
| 133 | Boards | `PUT` | `/api/boards/{boardId}/labels` | `add_board_label` | Add a board label if it does not already exist. | — | ✗ |
| 134 | Boards | `POST` | `/api/boards/{boardId}/copy` | `copy_board` | Copy a board. | — | ✗ |
| 135 | Boards | `POST` | `/api/boards/{boardId}/members/{memberId}` | `update_board_member_permissions` | Set a board member's role or individual permission flags. | — | ✗ |
| 136 | Boards | `GET` | `/api/boards/{boardId}/domains` | `get_board_domains` | List email domains a board is shared with. | — | ✗ |
| 137 | Boards | `POST` | `/api/boards/{boardId}/domains` | `add_board_domain` | Share a board with an email domain. | — | ✗ |
| 138 | Boards | `DELETE` | `/api/boards/{boardId}/domains/{domain}` | `delete_board_domain` | Stop sharing a board with an email domain. | — | ✗ |
| 139 | Boards | `GET` | `/api/boards/{boardId}/attachments` | `get_board_attachments` | List board attachment links using the legacy response shape. | — | ✗ |
| 140 | System | `GET` | `/schema-upgrade-status` | `get_schema_upgrade_status` | Get live startup schema-upgrade status. | — | ✗ |
