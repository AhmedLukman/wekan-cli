# Cards

The singular `wekan card` family covers only the five board/list-scoped core
CRUD operations:

```console
wekan card list <BOARD_ID> <LIST_ID>
wekan card get <BOARD_ID> <LIST_ID> <CARD_ID>
wekan card create <BOARD_ID> <LIST_ID> --title <TITLE> --swimlane-id <ID> [FIELDS...]
wekan card update <BOARD_ID> <LIST_ID> <CARD_ID> <FIELDS...>
wekan card delete <BOARD_ID> <LIST_ID> <CARD_ID> [--yes]
```

Linked-card creation, movement and dedicated ordering commands, archive and
restore, votes, poker, custom-field mutation, stickers, locations,
attachments, comments, and checklists are deliberately outside this family.

## Create and update fields

Create accepts `--description`, repeated `--member` and `--assignee`, and
`--received-at`, `--start-at`, `--due-at`, and `--end-at`. Dates must be
RFC3339 date-times. Repeated member and assignee flags supply the complete
initial arrays. Title and swimlane ID are required. Wekan v11.06 silently
ignores `parentId` in the body of this single-card creation endpoint, so the
CLI does not expose it.

Update accepts:

- `--title`, `--parent-id`, `--description`, `--color`, `--requested-by`, and
  `--assigned-by`;
- repeated `--label`, `--member`, and `--assignee` replacement arrays, or the
  corresponding `--clear-labels`, `--clear-members`, and
  `--clear-assignees` flags;
- RFC3339 date values, or one of `--clear-received-at`, `--clear-start-at`,
  `--clear-due-at`, and `--clear-end-at`;
- finite `--sort` and nonzero finite `--spent-time`;
- `--is-over-time <true|false>`; and
- `--due-complete <true|false>`.

At least one update field is required. Set and clear forms conflict. IDs and
text cannot be empty after trimming. Update titles are limited to 1000 UTF-16
code units. Colors use Wekan v11.06's card palette or exactly `#rrggbb`.

`--sort` and `--is-over-time` intentionally mirror broken Wekan v11.06
behavior. Wekan silently ignores `sort: 0` and `isOverTime: false`; a request
containing only those values can therefore return not found. It acknowledges
`isOverTime: true`, but targets the wrong document-field spelling, so the
card's real `is_overtime` value does not change in the pinned live stack.

Update results report submitted fields in this fixed order, regardless of option order:
`title`, `sort`, `parent_id`, `description`, `color`, `label_ids`, `requested_by`,
`assigned_by`, `received_at`, `start_at`, `due_at`, `end_at`, `spent_time`,
`is_over_time`, `members`, `assignees`, `due_complete`. These are the fields
sent to Wekan; they do not claim that Wekan persisted every value.

Wekan applies card-update fields independently, so it can persist an earlier
field before rejecting a later one. The CLI therefore reports
`outcome_unknown: true` when a dispatched card update fails and partial changes
may have occurred; it does not retry, fetch, or claim which fields changed.

## Stable output

Use `--output json` for stable machine-readable envelopes. Collection, create,
update, and delete data have these shapes:

```json
{"board_id":"board-id","list_id":"list-id","cards":[{"card_id":"card-id","title":"Todo","description":"Details","swimlane_id":"swimlane-id","received_at":null,"start_at":null,"due_at":null,"end_at":null,"assignees":[],"sort":0}]}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id"}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id","submitted_fields":["title","due_at"]}
{"board_id":"board-id","list_id":"list-id","card_id":"card-id","deleted":true,"delete_mode":"hard"}
```

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
