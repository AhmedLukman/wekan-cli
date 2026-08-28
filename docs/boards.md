# Boards

The authenticated `wekan board` family implements the core Wekan v11.06 board
lifecycle:

```text
wekan board list [--public]
wekan board count
wekan board get <BOARD_ID>
wekan board create --title <TITLE> [--owner <USER_ID>] [--permission private|public] [--color <COLOR>] [--no-comments] [--comment-only] [--worker]
wekan board rename <BOARD_ID> --title <TITLE>
wekan board delete <BOARD_ID> [--yes]
```

Every command requires a selected profile and an unexpired stored credential.
Board imports, copies, labels, memberships, domains, attachments, card settings,
and other subresources are outside this command family's scope.

## Listing and counting

`board list` reuses Wekan's user-board endpoint to return the authenticated
user's active, non-archived, non-internal boards. `board list --public` instead
returns the server-wide public-board projection. Both scopes contain only board
ID and title and use an empty array when no boards match.

`board count` returns Wekan's server-wide private and public counts. These are
not counts of boards visible to the authenticated caller.

## Creating and renaming

`--title` must be non-empty. Omitting `--owner` lets Wekan select the
authenticated caller. Permission defaults to `private`, and color defaults to
`belize`. Accepted v11.06 colors are `belize`, `nephritis`, `pomegranate`,
`pumpkin`, `wisteria`, `moderatepink`, `strongcyan`, `limegreen`, `midnight`,
`dark`, `relax`, `corteza`, `appleglasspastel`, `clearblue`, `cleargreen`,
`clearorange`, `clearpink`, `clearpurple`, `clearred`, `natural`, `modern`,
`moderndark`, `exodark`, `cleandark`, and `cleanlight`.

`--no-comments`, `--comment-only`, and `--worker` independently set the matching
role flags on the initial member. Wekan v11.06 always makes that member active
and an administrator, so the CLI does not expose ineffective `isActive` or
`isAdmin` controls. `rename` also requires a non-empty title and verifies both
the ID and title returned by Wekan.

## Board detail

`board get` returns the complete normalized v11.06 board model. Names are
snake_case, `_id` becomes `board_id`, `backgroundImageURL` becomes
`background_image_url`, and the model's `type` becomes `board_type`. It includes
identity, timestamps, ordering, typed members, sharing records, labels,
watcher records, organization/team/domain records, theme/background values, date defaults,
card-display controls, and board settings.

Known enum fields accept only their corrected v11.06 values, and timestamp
fields must contain RFC 3339 date-times. Other values cause a protocol error.

Known omitted scalar fields are JSON `null`, and optional omitted collections
are empty arrays. Explicitly null collections are rejected because the corrected
v11.06 contract defines them as arrays. `members` is required and is never
normalized from null or omission. The board ID and title must be non-empty.
Unknown top-level and nested fields cause a protocol error and are not included
in JSON or human output. Human detail output groups the modeled fields and
escapes terminal control characters.

When Wekan reports that the requested board does not exist, `board get` returns
the stable `not_found` error code with exit status 5. The error details preserve
the outer HTTP status and Wekan's embedded status and reason.

## Deleting safely

Deletion permanently removes the board and its contents. Interactive human
execution displays that warning and defaults to no. Declining is an exit-0
cancellation and sends no HTTP request. JSON or non-terminal execution must pass
command-local `--yes`; other board commands do not accept it.

The CLI verifies the returned board ID. Board mutations are never retried, and
ambiguous transport, redirect, server, or successful-body failures report
`outcome_unknown: true` because Wekan may already have applied the operation.

## Stable JSON data shapes

Successful commands use the standard `{ "ok": true, "data": ... }` envelope.
Their `data` values are:

```json
{"scope":"active","boards":[{"board_id":"board-id","title":"Planning"}]}
{"private":2,"public":1}
{"board_id":"board-id","default_swimlane_id":"swimlane-id"}
{"board_id":"board-id","title":"Renamed"}
{"board_id":"board-id","deleted":true}
```

The list scope is `active` or `public`. `board get` uses the comprehensive typed
document described above.
