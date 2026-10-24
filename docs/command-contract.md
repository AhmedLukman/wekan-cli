# Command behavior and inventory

This reference summarizes the current `wekan` command interface verified
against Wekan v12.57. The family documentation records command syntax,
response fields, and native server limitations. Typed commands reject unmapped
upstream object fields with `protocol_error`.

## JSON presence and nullability

Successful structured output is one `{"ok":true,"data":...}` object on stdout.
Failures are one `{"ok":false,"error":{"code":...,"message":...,"details":{...}}}`
object on stderr. Each ends with a newline; the other stream is empty except
for documented interactive prompts and output-delivery failures. `--help` and
`--version` print text even when JSON is selected. See the
[agent contract](agent-contract.md) for shared delivery, decoding, mutation, and
error rules; resource-family guides define their result fields and raw output.

The [JSON presence and nullability contract](agent-contract.md#json-presence-and-nullability)
defines collection shapes and the distinctions between null, omission, and
empty values, including command-specific exceptions.

## Errors and exit statuses

The [error catalogue](agent-contract.md#errors) lists every current code and
its exit status. Exit 0 includes explicitly cancelled no-ops. A successful HTTP
status alone does not establish CLI success: typed decoding, embedded Wekan
errors, or partial batch outcomes can still fail. Mutations report completed
actions where the pinned server supports that claim. Endpoints that can accept
an ineffective request use request-accepted wording. Human messages explain concrete limitations where needed.
Delayed ownership transfer reports that the request was accepted. Inspect error details before deciding whether to retry a
failed request.

| Affected command | What its output establishes |
| --- | --- |
| `user create` | Returns the newly created user ID; duplicate identities fail. |
| `user token create` | Returns a token for an existing user and records the required audit reason; a missing user returns HTTP 404. |
| `user token delete --token-stdin` | Acknowledges raw issued tokens without revoking them (W11-058). Delete-all revokes all sessions for the selected user. |
| `user disable-login` | Reports the returned login-disabled flag or an unchanged user for self-targeting. Disabled accounts cannot start new REST sessions. |
| `attachment upload` | Request accepted; `upload_confirmed: false` because deferred native validation can still remove the attachment. Receipt text is named `server_message`. |
| `attachment upload-background --transport raw` | The known helper failure returns `server_error` with native HTTP 401 after storing the upload. It does not establish credential rejection or a changed background. |
| Board/card/list/swimlane copy | Copy completed, including native attachment copying. |
| Cross-board list move | Request accepted; source list remains, position is ignored, and populated moves can fail after transferring the first card. Same-board list moves retain direct success wording. |
| List/card/swimlane/custom-field/dropdown-item/integration deletion | Request accepted even when nothing matched. List, card, and swimlane commands use `deletion_requested: true`, without `deleted: true`. Wrong-list card deletion can remove children while leaving the card. |
| Card/custom-field/dropdown-item/integration update; dropdown-item creation | Request accepted; missing targets can cause a successful no-op. Human card-update output explains ineffective overtime changes when requested; submitted fields describe input rather than applied changes. |
| Dependency update/delete | Request accepted; a missing edge is a successful no-op. |
| Board member add/remove | Request accepted; active-member addition can preserve permissions, and sole-administrator removal can leave membership intact. |

`protocol_error` uses exit 4 for protocol/response validation failures and
exit 5 for documented HTTP-200 Wekan error envelopes without an application
status and for native export text failures. `internal_error` uses exit 1 for
unexpected internal failures, including initialization, and exit 4 for local
output delivery failures. Error codes and exit statuses therefore describe
related but distinct information; neither alone
uniquely determines the other. `credential_not_found` uses exit 5 when no
credential exists initially, and exit 6 when the credential used to prepare a
mutation disappears before execution. Unavailable error details never establish a
successful or safe-to-retry outcome.

## Complete command inventory

Every leaf command below is included in the verification inventory. A command can select several
native endpoints or output shapes through its arguments. The
[API coverage table](api-coverage.md) records all 168 corrected native operations
(167 implemented and the Amazon SES endpoint intentionally excluded), with
complete options, requirements, conflicts, and endpoint selectors. The six
local profile commands, two local guidance commands, and the raw API escape
hatch are included below too.
The executable-wide contract test discovers the parser tree and requires
this inventory to cover every leaf; it also checks help and JSON usage errors
at every command/group depth. Family documentation specifies each success
shape and verified native limitations.

| Command | Detailed contract |
| --- | --- |
| `wekan admin attachment-settings get` | [Contract](admin-settings.md) |
| `wekan admin attachment-settings update` | [Contract](admin-settings.md) |
| `wekan admin domain list` | [Contract](admin.md) |
| `wekan admin oauth-provider get` | [Contract](admin.md) |
| `wekan admin oauth-provider update` | [Contract](admin.md) |
| `wekan admin org features update` | [Contract](admin.md) |
| `wekan admin org features update-all` | [Contract](admin.md) |
| `wekan admin org list` | [Contract](admin.md) |
| `wekan admin passwordless update` | [Contract](admin.md) |
| `wekan admin problem acknowledge` | [Contract](admin.md) |
| `wekan admin problem get` | [Contract](admin.md) |
| `wekan admin problem list` | [Contract](admin.md) |
| `wekan admin settings get` | [Contract](admin-settings.md) |
| `wekan admin settings update` | [Contract](admin-settings.md) |
| `wekan admin team features update` | [Contract](admin.md) |
| `wekan admin team features update-all` | [Contract](admin.md) |
| `wekan admin team list` | [Contract](admin.md) |
| `wekan api request` | [Contract](api.md) |
| `wekan attachment copy` | [Contract](attachments.md) |
| `wekan attachment delete` | [Contract](attachments.md) |
| `wekan attachment download` | [Contract](attachments.md) |
| `wekan attachment download-background` | [Contract](attachments.md) |
| `wekan attachment export` | [Contract](exports.md) |
| `wekan attachment get` | [Contract](attachments.md) |
| `wekan attachment list` | [Contract](attachments.md) |
| `wekan attachment list-deleted` | [Contract](attachments.md) |
| `wekan attachment move` | [Contract](attachments.md) |
| `wekan attachment restore` | [Contract](attachments.md) |
| `wekan attachment upload` | [Contract](attachments.md) |
| `wekan attachment upload-background` | [Contract](attachments.md) |
| `wekan auth login` | [Contract](auth.md) |
| `wekan auth logout` | [Contract](auth.md) |
| `wekan auth preflight` | [Contract](auth.md) |
| `wekan auth register` | [Contract](auth.md) |
| `wekan auth status` | [Contract](auth.md) |
| `wekan board attachment list` | [Contract](boards.md) |
| `wekan board card-field-order get` | [Contract](boards.md) |
| `wekan board card-field-order update` | [Contract](boards.md) |
| `wekan board card-settings get` | [Contract](boards.md) |
| `wekan board card-settings update` | [Contract](boards.md) |
| `wekan board chart export` | [Contract](boards.md) |
| `wekan board copy` | [Contract](copy-and-move.md) |
| `wekan board count` | [Contract](boards.md) |
| `wekan board create` | [Contract](boards.md) |
| `wekan board delete` | [Contract](boards.md) |
| `wekan board domain add` | [Contract](boards.md) |
| `wekan board domain list` | [Contract](boards.md) |
| `wekan board domain remove` | [Contract](boards.md) |
| `wekan board export` | [Contract](exports.md) |
| `wekan board export-all` | [Contract](boards.md) |
| `wekan board get` | [Contract](boards.md) |
| `wekan board import` | [Contract](imports.md) |
| `wekan board import-trello` | [Contract](imports.md) |
| `wekan board import-zip` | [Contract](imports.md) |
| `wekan board label add` | [Contract](bulk-cards-and-labels.md) |
| `wekan board label list` | [Contract](bulk-cards-and-labels.md) |
| `wekan board list` | [Contract](boards.md) |
| `wekan board member add` | [Contract](memberships.md) |
| `wekan board member list` | [Contract](memberships.md) |
| `wekan board member remove` | [Contract](memberships.md) |
| `wekan board member update` | [Contract](memberships.md) |
| `wekan board rename` | [Contract](boards.md) |
| `wekan board view-settings get` | [Contract](boards.md) |
| `wekan board view-settings update` | [Contract](boards.md) |
| `wekan card archive` | [Contract](cards.md) |
| `wekan card assignee add` | [Contract](memberships.md) |
| `wekan card assignee list` | [Contract](memberships.md) |
| `wekan card assignee remove` | [Contract](memberships.md) |
| `wekan card bot-run report` | [Contract](cards.md) |
| `wekan card bulk create` | [Contract](bulk-cards-and-labels.md) |
| `wekan card bulk delete` | [Contract](bulk-cards-and-labels.md) |
| `wekan card bulk labels` | [Contract](bulk-cards-and-labels.md) |
| `wekan card copy` | [Contract](copy-and-move.md) |
| `wekan card count` | [Contract](cards.md) |
| `wekan card create` | [Contract](cards.md) |
| `wekan card delete` | [Contract](cards.md) |
| `wekan card export` | [Contract](exports.md) |
| `wekan card find-by-custom-field` | [Contract](cards.md) |
| `wekan card get` | [Contract](cards.md) |
| `wekan card import-ics` | [Contract](imports.md) |
| `wekan card label list` | [Contract](bulk-cards-and-labels.md) |
| `wekan card list` | [Contract](cards.md) |
| `wekan card member add` | [Contract](memberships.md) |
| `wekan card member list` | [Contract](memberships.md) |
| `wekan card member remove` | [Contract](memberships.md) |
| `wekan card restore` | [Contract](cards.md) |
| `wekan card set-custom-field` | [Contract](cards.md) |
| `wekan card update` | [Contract](cards.md) |
| `wekan checklist create` | [Contract](checklists.md) |
| `wekan checklist delete` | [Contract](checklists.md) |
| `wekan checklist get` | [Contract](checklists.md) |
| `wekan checklist item create` | [Contract](checklists.md) |
| `wekan checklist item delete` | [Contract](checklists.md) |
| `wekan checklist item get` | [Contract](checklists.md) |
| `wekan checklist item update` | [Contract](checklists.md) |
| `wekan checklist list` | [Contract](checklists.md) |
| `wekan checklist update` | [Contract](checklists.md) |
| `wekan comment create` | [Contract](comments.md) |
| `wekan comment delete` | [Contract](comments.md) |
| `wekan comment get` | [Contract](comments.md) |
| `wekan comment list` | [Contract](comments.md) |
| `wekan comment update` | [Contract](comments.md) |
| `wekan custom-field create` | [Contract](custom-fields.md) |
| `wekan custom-field delete` | [Contract](custom-fields.md) |
| `wekan custom-field get` | [Contract](custom-fields.md) |
| `wekan custom-field item create` | [Contract](custom-fields.md) |
| `wekan custom-field item delete` | [Contract](custom-fields.md) |
| `wekan custom-field item update` | [Contract](custom-fields.md) |
| `wekan custom-field list` | [Contract](custom-fields.md) |
| `wekan custom-field update` | [Contract](custom-fields.md) |
| `wekan dependency create` | [Contract](dependencies.md) |
| `wekan dependency delete` | [Contract](dependencies.md) |
| `wekan dependency list` | [Contract](dependencies.md) |
| `wekan dependency update` | [Contract](dependencies.md) |
| `wekan integration activity add` | [Contract](integrations.md) |
| `wekan integration activity remove` | [Contract](integrations.md) |
| `wekan integration create` | [Contract](integrations.md) |
| `wekan integration delete` | [Contract](integrations.md) |
| `wekan integration get` | [Contract](integrations.md) |
| `wekan integration inbound-email bounce` | [Contract](integrations.md) |
| `wekan integration inbound-email submit` | [Contract](integrations.md) |
| `wekan integration list` | [Contract](integrations.md) |
| `wekan integration update` | [Contract](integrations.md) |
| `wekan list archive` | [Contract](lists.md) |
| `wekan list copy` | [Contract](copy-and-move.md) |
| `wekan list create` | [Contract](lists.md) |
| `wekan list delete` | [Contract](lists.md) |
| `wekan list get` | [Contract](lists.md) |
| `wekan list list` | [Contract](lists.md) |
| `wekan list move` | [Contract](copy-and-move.md) |
| `wekan list restore` | [Contract](lists.md) |
| `wekan list update` | [Contract](lists.md) |
| `wekan profile add` | [Contract](config.md) |
| `wekan profile list` | [Contract](config.md) |
| `wekan profile remove` | [Contract](config.md) |
| `wekan profile show` | [Contract](config.md) |
| `wekan profile update` | [Contract](config.md) |
| `wekan profile use` | [Contract](config.md) |
| `wekan rule create` | [Contract](rules.md) |
| `wekan rule delete` | [Contract](rules.md) |
| `wekan rule get` | [Contract](rules.md) |
| `wekan rule list` | [Contract](rules.md) |
| `wekan rule update` | [Contract](rules.md) |
| `wekan skill list` | [Contract](skills.md) |
| `wekan skill show` | [Contract](skills.md) |
| `wekan swimlane copy` | [Contract](copy-and-move.md) |
| `wekan swimlane create` | [Contract](swimlanes.md) |
| `wekan swimlane delete` | [Contract](swimlanes.md) |
| `wekan swimlane get` | [Contract](swimlanes.md) |
| `wekan swimlane list` | [Contract](swimlanes.md) |
| `wekan swimlane move` | [Contract](copy-and-move.md) |
| `wekan swimlane update` | [Contract](swimlanes.md) |
| `wekan system schema-upgrade-status` | [Contract](system.md) |
| `wekan user boards` | [Contract](users.md) |
| `wekan user cards` | [Contract](users.md) |
| `wekan user create` | [Contract](users.md) |
| `wekan user current` | [Contract](users.md) |
| `wekan user delete` | [Contract](users.md) |
| `wekan user disable-login` | [Contract](users.md) |
| `wekan user enable-login` | [Contract](users.md) |
| `wekan user get` | [Contract](users.md) |
| `wekan user list` | [Contract](users.md) |
| `wekan user take-ownership` | [Contract](users.md) |
| `wekan user token create` | [Contract](users.md) |
| `wekan user token delete` | [Contract](users.md) |
| `wekan user update` | [Contract](users.md) |
