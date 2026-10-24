# Copy and move

These commands send one native Wekan v12.57 request. They execute directly and
support human or JSON output, named profiles, and the usual global options.

```console
wekan board copy <BOARD_ID> [--title <TEXT>]
wekan card copy <CARD_ID> --board <BOARD_ID> --list <LIST_ID> --target-swimlane-id <ID> [--target-board <ID>] [--target-list <ID>] [--position <NUMBER> | --position-json <JSON>]
wekan list copy <LIST_ID> --board <BOARD_ID> [--target-board <ID>] [--target-swimlane-id <ID>] [--position <NUMBER> | --position-json <JSON>]
wekan list move <LIST_ID> --board <BOARD_ID> [--target-board <ID>] [--target-swimlane-id <ID>] [--position <NUMBER> | --position-json <JSON>]
wekan swimlane copy <SWIMLANE_ID> --board <BOARD_ID> [--target-board <ID>] [--title <TEXT>] [--position <NUMBER> | --position-json <JSON>]
wekan swimlane move <SWIMLANE_ID> --board <BOARD_ID> [--target-board <ID>] [--title <TEXT>] [--position <NUMBER> | --position-json <JSON>]
```

Destination board defaults to the source board. Card copying additionally
defaults the destination list to the source list, even when changing boards;
specify `--target-list` explicitly when that is not intended. Its destination
swimlane is always required. Optional selectors are omitted from the request,
leaving Wekan responsible for their defaults. IDs are trimmed and cannot be
blank, `.` or `..`. Titles are sent unchanged. Board copying requires board
administrator access; other transfers require write access to both boards.

## Position inputs

`--position` accepts finite numbers, including negative and fractional values.
Wekan converts positions to numbers, floors fractions, and clamps negatives to
zero; non-finite converted values and values past the end append. Positions
count from zero: cards from the top of the destination list, lists from the
left of the destination board, and swimlanes from the top of the board.

`--position-json` conflicts with `--position` and preserves the supplied JSON
type. Card copying accepts a number, string, or null. Lists and swimlanes
accept any JSON value, including booleans, arrays, and objects. Explicit null
positions a copied card at the top, but skips list/swimlane repositioning.
Omitting both flags leaves the server's original copy/move sorting behavior.

## Results and pinned behavior

JSON uses the standard `{ "ok": true, "data": ... }` envelope. Result fields are:

| Operation | Data fields |
| --- | --- |
| Board copy | `source_board_id`, new `board_id` |
| Card copy | `source_board_id`, `source_list_id`, `source_card_id`, resolved `target_board_id`, `target_list_id`, `target_swimlane_id`, new `card_id` |
| List copy/move | `source_board_id`, `source_list_id`, `target_board_id`, nullable `target_swimlane_id`, `returned_list_id` |
| Swimlane copy/move | `source_board_id`, `source_swimlane_id`, `target_board_id`, `returned_swimlane_id` |

Human output reports the completed copy or move, subject to the accepted-request
wording for cross-board list moves described below.

Wekan v12.57 creates a new list for same-board list copies. Cross-board copies
can reuse a same-title destination list. Copying includes every source-list card
across its swimlanes, and an omitted destination swimlane now uses native
selection rules instead of the old missing-swimlane failure.

Same-board list moves reposition the existing list and can change its stored
swimlane ID. Cross-board moves retain the source list and ignore position.
The old destination-swimlane source filter is fixed, but a populated move still
moves its first card before `Meteor.userId()` throws outside a DDP invocation,
returning HTTP 500. Destination lists may already exist after that failure.
Labels map to existing destination labels by name; unmatched labels are dropped.

A populated cross-board swimlane move can create/reuse destination lists and
move a card before the same `Meteor.userId()` failure returns HTTP 500. The
swimlane itself remains on the source board because its update runs after the
card transfers. Empty swimlane transfers can complete successfully.

A successful swimlane move also starts an unawaited insertion of a default
swimlane whose ID is `<BOARD_ID>-default`, even when active swimlanes already
exist. If that ID is absent, the extra lane appears with sort zero and may
change the board's order after the response. Repeated moves reuse that ID.

Copies and moves may fail after partial changes. Errors preserve the server
status and error details. The
CLI does not retry or repair these requests. Acknowledge the failure and inspect
source and destination state before deciding on another mutation.

Card copying delegates subtasks, checklists/items, comments, and attachment
bytes to Wekan. W11-047 is fixed: card, list, swimlane, and board copies await
attachment copying and create independently downloadable files. Live regressions
start with an actual native upload and compare every copied file's bytes.

Cross-board list moves report request acceptance and state that the source list
is retained and position ignored. JSON returns scoped resource IDs; the CLI does
not add a shared warning envelope or post-verify completed writes.
