# Cards

The singular `wekan card` family covers the five board/list-scoped core
CRUD operations:

```console
wekan card list --board <BOARD_ID> --list <LIST_ID>
wekan card get <CARD_ID> --board <BOARD_ID> --list <LIST_ID>
wekan card create --board <BOARD_ID> --list <LIST_ID> --title <TITLE> --swimlane-id <ID> [FIELDS...]
wekan card update <CARD_ID> --board <BOARD_ID> --list <LIST_ID> <FIELDS...>
wekan card delete <CARD_ID> --board <BOARD_ID> --list <LIST_ID> [--yes]
```

Create also supports linked cards; update supports movement, archive/restore,
votes, poker, custom-field values, stickers, and locations. Dedicated ordering
endpoints, attachments, comments, and checklists are outside this family.

## Create and update fields

Create accepts `--description`, repeated `--member` and `--assignee`, and
`--received-at`, `--start-at`, `--due-at`, and `--end-at`. Dates must be
RFC3339 date-times. Repeated member and assignee flags supply the complete
initial arrays. Swimlane ID is required. Title is required unless `--linked-id`
selects an existing source card. Wekan checks read access to the source board
and ignores other content fields when creating a linked card. Wekan v11.06
silently ignores `parentId` in the body of this single-card creation endpoint, so the
CLI does not expose it.

Update accepts:

- `--title`, `--parent-id`, `--description`, `--color`, `--requested-by`, and
  `--assigned-by`;
- repeated `--label`, `--member`, and `--assignee` replacement arrays, or the
  corresponding `--clear-labels`, `--clear-members`, and
  `--clear-assignees` flags;
- RFC3339 date values, or one of `--clear-received-at`, `--clear-start-at`,
  `--clear-due-at`, and `--clear-end-at`;
- finite `--sort` and `--spent-time`;
- `--is-over-time <true|false>`;
- `--due-complete <true|false>` and `--archive <true|false>`;
- `--swimlane-id` and `--list-id` for movement within the source board;
- `--new-board-id`, `--new-swimlane-id`, and `--new-list-id` for movement
  between boards; and
- `--vote`, `--poker`, `--custom-fields`, `--stickers`, and `--locations`
  containing JSON with the fields described below.

At least one update field is required. Set and clear forms conflict. IDs and
text cannot be empty after trimming. Colors use Wekan v11.06's card palette or
exactly `#rrggbb`.

Update values intentionally mirror broken Wekan v11.06 behavior. Wekan
truncates titles after the first 1000 JavaScript UTF-16 code units. It silently
ignores `sort: 0`, `spentTime: 0`, and `isOverTime: false`; a request containing
only those values can therefore return not found. It acknowledges
`isOverTime: true`, but targets the wrong document-field spelling, so the
card's real `is_overtime` value does not change in the pinned live stack.

Update results report submitted fields in this fixed order, regardless of option order:
`title`, `sort`, `parent_id`, `description`, `color`, `label_ids`, `requested_by`,
`assigned_by`, `received_at`, `start_at`, `due_at`, `end_at`, `spent_time`,
`is_over_time`, `members`, `assignees`, `due_complete`, `swimlane_id`, `list_id`,
`new_board_id`, `new_swimlane_id`, `new_list_id`, `archive`, `vote`, `poker`,
`custom_fields`, `stickers`, `locations`. These are the fields
sent to Wekan; they do not claim that Wekan persisted every value.

Wekan applies card-update fields independently, so it can persist an earlier
field before rejecting a later one. The CLI therefore reports
`outcome_unknown: true` when a dispatched card update fails and partial changes
may have occurred; it does not retry, fetch, or claim which fields changed.

## Movement and structured fields

`--board` and `--list` identify the card's source scope; `--list-id` identifies
its destination list. Supply all three `--new-*-id` flags for a cross-board
move. Wekan ignores an incomplete cross-board destination and ignores
same-board movement fields during a complete cross-board move. Moves require
write access to the destination board and place the card at the top of its list.
Wekan applies `--due-complete`, `--stickers`, and `--locations` after movement
using the original scope, so combining them with a list or board move can return
success without saving those later fields. Use separate requests to save them
after a move.

`--archive true` archives and `--archive false` restores. Wekan hides archived
cards from scoped `card get`; restore selects by board and card ID independently
of the supplied list.

Structured flags accept one JSON object or array, using Wekan's wire field names:

| Flag | JSON shape and supported fields |
|---|---|
| `--vote` | Object: `question` (string), `positive`/`negative` (string arrays), `end` (RFC3339 string or null), `public`/`allowNonBoardMembers` (booleans). Wekan requires a question and defaults omitted booleans to false. |
| `--poker` | Object: `question` (boolean or null), `one`, `two`, `three`, `five`, `eight`, `thirteen`, `twenty`, `forty`, `oneHundred`, `unsure` (string arrays or null), `end` (RFC3339 string or null), `allowNonBoardMembers` (boolean or null), `estimation` (number or null). Omitted `allowNonBoardMembers` defaults to false. |
| `--custom-fields` | Array of objects: `_id` (string or null), `value` (string, number, boolean, string array with optional null members, or null). Both keys may be omitted. |
| `--stickers` | Array of objects: optional `icon`/`name` (strings), `highlight` (`underline` or `round`), `position` (number). |
| `--locations` | Array of objects: optional `_id`/`name`/`address` (strings or null), `latitude`/`longitude` (numbers or null). Wekan generates omitted IDs, defaults omitted names/addresses to empty strings, and converts null coordinates to zero. |

Unknown keys and invalid types fail argument parsing. Vote and poker objects
replace their settings; Wekan resets every voter array regardless of supplied
values. Custom fields, stickers, and locations replace their complete arrays;
`[]` clears them. Explicit null values are preserved.

```console
wekan card create --board board-id --list list-id --swimlane-id lane-id --linked-id source-card-id
wekan card update card-id --board board-id --list list-id --list-id destination-list-id
wekan card update card-id --board board-id --list list-id --custom-fields '[{"_id":"field-id","value":"Ready"}]'
```

## Stable output

Use `--output json` for stable machine-readable envelopes. Collection, create,
update, and delete data have these shapes:

```json
{"board_id":"board-id","list_id":"list-id","cards":[{"card_id":"card-id","title":"Todo","description":"Details","swimlane_id":"swimlane-id","received_at":null,"start_at":null,"due_at":null,"end_at":null,"assignees":[],"sort":0}]}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id"}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id","submitted_fields":["title","due_at"]}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id","deleted":true,"delete_mode":"hard"}
```

Update result board/list IDs identify the request's source scope, including after
movement.

These objects appear under the usual `{"ok":true,"data":...}` envelope.
`card get` returns the complete supported Wekan v11.06 card document in snake
case. Omitted collections become arrays and omitted optional scalars become
JSON `null`. The card, custom-field, sticker, location, dependency, vote, and
poker objects are strict; unknown fields are protocol errors. Known IDs,
timestamps, colors, card types, sticker highlights, nested values, and required
types are also validated.

A missing scoped card produces an empty HTTP 200 response in Wekan v11.06. The
CLI normalizes that response to `not_found`.

## Hard deletion warning

`card delete` requires interactive confirmation or command-local `--yes` and
issues exactly one DELETE request. The endpoint is idempotent and always
returns the requested ID, including for an absent card. Therefore
`deleted: true` means Wekan accepted the request; it does not prove that the
card previously existed or was removed.

Always supply the card's actual list ID. Wekan v11.06 looks up the card by board
and card ID before cascading child deletion, but includes the list ID only when
removing the card itself. A wrong list ID can therefore delete the card's child
resources while leaving the card intact. The CLI mirrors the endpoint and does
not preflight or post-verify existence.
