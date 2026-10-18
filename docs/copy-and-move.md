# Copy and move

These commands send one native Wekan v11.06 request. They execute directly and
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

JSON uses the standard `{ "ok": true, "data": ... }` envelope. Copy results
include source identifiers and the returned `board_id` or `card_id`; card copies
also include resolved destination selectors. List and swimlane transfers include
source and destination identifiers and a `returned_list_id` or
`returned_swimlane_id`. Human output reports the completed copy or move.

Wekan v11.06 reuses an active destination list with the same title when copying
a list. A same-board list copy can therefore return the source ID and duplicate
its cards into that same list. List copying selects source cards using the
source list's stored swimlane; it does not aggregate cards from every swimlane.
When selected cards need a destination swimlane, omitting `--target-swimlane-id`
can return HTML HTTP 500 (`Swimlane ID is required`). Empty or no-matching-card
copies can behave differently. Explicit destination swimlanes avoid that specific
failure; the CLI sends the requested selectors without adding a destination.

Same-board list moves reposition the existing list and can change its stored
swimlane ID. Cross-board list moves retain the source list, ignore position,
and return the source list ID. When no same-title destination list exists, a
requested destination swimlane also filters source cards, potentially selecting
none. If a same-title destination list already exists, Wekan first attempts to
move source cards without that filter, even when none match the requested
swimlane. A later request can therefore move cards left untouched by an earlier
request that created the destination list. Selected cards can be moved before
the server throws `Meteor.userId()` outside a DDP invocation and returns HTML
HTTP 500. Labels on transferred cards map to existing destination labels by
name; unmatched labels are dropped.

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

Card copying delegates subtasks, checklists/items, and comments to Wekan.
In v11.06, card/list/swimlane/board copies return HTTP 200 and copy the cards,
but omit their attachments: the unawaited `copyFile` helper fails with
`ReferenceError: ReactiveCache is not defined` before creating metadata or
copying bytes. The source attachments remain readable. A successful copy result
therefore does not confirm attachment duplication. This server defect is
regression-tested without a CLI workaround.

Human copy output states that Wekan v11.06 omits attachments. Cross-board list moves report request acceptance and state that the source list is retained and position ignored; same-board moves retain direct success wording. JSON returns the scoped resource IDs, without a shared warning envelope.
