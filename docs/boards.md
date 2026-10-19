# Boards

The authenticated `wekan board` family implements the core Wekan v11.06 board
lifecycle:

```text
wekan board list [--public]
wekan board count
wekan board get <BOARD_ID>
wekan board create --title <TITLE> [--owner <USER_ID>] [--is-admin <true|false>] [--is-active <true|false>] [--permission private|public] [--color <COLOR>] [--no-comments] [--comment-only] [--worker]
wekan board rename <BOARD_ID> --title <TITLE>
wekan board copy <BOARD_ID> [--title <TEXT>]
wekan board delete <BOARD_ID> [--yes]
```

Every command requires a selected profile and an unexpired stored credential.
Board imports, attachments,
and other subresources are outside this command family's scope.

`board copy <BOARD_ID> [--title <TEXT>]` uses Wekan's native board copier.
`board label list/add` manages reusable labels; see [bulk cards and labels](bulk-cards-and-labels.md).

`board member list/add/remove/update` implements board membership management.
See [board and card memberships](memberships.md) for syntax, permission inputs,
stable results, and Wekan's no-op behavior.

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
role flags on the initial member. `--is-admin` and `--is-active` both default to true. False values use Wekan's
accepted string representation because its route ignores literal boolean false.
An inactive initial member may be unable to read the newly created board. `rename` also requires a non-empty title and verifies both
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
cancellation with operation `board_delete` and sends no HTTP request. JSON or non-terminal execution must pass
command-local `--yes`; other board commands do not accept it.

The CLI verifies the returned board ID. Board mutations are never retried, and
transport, redirect, server, and response-validation failures retain their
actual error codes and diagnostics.

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

Copy/move command syntax, position inputs, typed acknowledgments, and pinned
server behavior are documented in [copy and move](copy-and-move.md).

## Card settings and aging

`board card-settings get --board <BOARD_ID>` reads the native card-settings
projection. `board card-settings update --board <BOARD_ID>` requires at least
one setting flag. Boolean values are explicit `true` or `false`; omission
preserves the existing setting, and explicit false or zero is sent unchanged.

```console
wekan board card-settings get --board BOARD_ID
wekan board card-settings update --board BOARD_ID --allows-attachments false --allows-creator-on-minicard true
wekan --output json board card-settings update --board BOARD_ID --card-aging true --card-aging-days1 7 --card-aging-days2 14 --card-aging-days3 28
```

All boolean flags accept `<true|false>`:

- `--allows-card-counter-list`
- `--card-aging`
- `--allows-board-member-list`
- `--allows-show-lists`
- `--allows-attachments`
- `--allows-checklists`
- `--allows-comments`
- `--allows-description-title`
- `--allows-description-text`
- `--allows-activities`
- `--allows-labels`
- `--allows-creator`
- `--allows-assignee`
- `--allows-members`
- `--allows-requested-by`
- `--allows-card-sorting-by-number`
- `--allows-card-number`
- `--allows-assigned-by`
- `--allows-received-date`
- `--allows-start-date`
- `--allows-end-date`
- `--allows-due-date`
- `--allows-subtasks`
- `--allows-creator-on-minicard`
- `--allows-attachments-on-minicard`
- `--allows-checklists-on-minicard`
- `--allows-checklist-at-minicard`
- `--allows-cover-attachment-on-minicard`
- `--allows-badge-attachment-on-minicard`
- `--allows-card-sorting-by-number-on-minicard`
- `--allows-card-number-on-minicard`
- `--allows-description-title-on-minicard`
- `--allows-description-text-on-minicard`
- `--allows-labels-on-minicard`
- `--allows-assignee-on-minicard`
- `--allows-members-on-minicard`
- `--allows-requested-by-on-minicard`
- `--allows-assigned-by-on-minicard`
- `--allows-received-date-on-minicard`
- `--allows-start-date-on-minicard`
- `--allows-end-date-on-minicard`
- `--allows-due-date-on-minicard`
- `--allows-subtasks-on-minicard`
- `--allows-show-lists-on-minicard`

`--card-aging-days1`, `--card-aging-days2`, and `--card-aging-days3` accept
integer days from 0 through 9007199254740991 (JavaScript's maximum safe integer).
Negative, fractional, and larger inputs fail locally. Threshold order is not
restricted. Flags cannot repeat. Use `wekan api request` for Wekan's broader
string/JSON input coercions.

Both reads and updates return `board_id` and a complete `settings` object;
updates also return `submitted_fields` in corrected-contract order. Settings
keys match the snake_case names in `board get`. Omitted response properties
appear as JSON null; explicit null, incorrect types, and unknown properties
are protocol errors. Stored numeric thresholds can be fractional or negative
and are returned without coercion. Human output includes every setting.

Reading uses Wekan's board-access check; updating uses its board-write check.
Normal members can update settings, read-only members can read them, and site
administrators pass both checks. Wekan can return HTML permission or
missing-board errors on these routes; the CLI preserves their HTTP classification.

## Domain sharing

```console
wekan board domain list --board BOARD_ID
wekan board domain add example.com --board BOARD_ID
wekan board domain remove example.com --board BOARD_ID
```

Domain listing uses the dedicated endpoint and requires a **site administrator**,
including for a public board or a board administered by the caller. It includes
inactive records in stored order. Regular callers receive Wekan's HTTP 200
serialized Forbidden error, mapped to `permission_denied` with the embedded 403.

Addition sends the domain unchanged as JSON. Removal encodes the supplied value
as one path segment and rejects empty values and the exact values `.` and `..`
before HTTP. Wekan handles trimming, lowercasing, and validation. Removal runs
directly without confirmation or `--yes`.

**Pinned Wekan v11.06 defect:** valid board/site-administrator domain additions
and removals fail with HTTP 200 `not-authorized` because `setBoardDomains`
receives no DDP user identity. Sharing remains unchanged, including when adding
an existing domain or removing an absent one. The CLI returns `protocol_error`
with exit 5 and preserves `http_status`, `server_error`, `server_reason`,
`server_message`, `server_error_type`, and `server_is_client_safe`.
It sends the native request without a workaround or retry.
See the [pinned route and method implementation](https://github.com/wekan/wekan/blob/bee4e71d86d0ef2669f9a389d77aba03fdb8103f/server/models/boards.js).

Domain list success contains `{board_id, domains}`. Each record has `domain`
and `is_active`. Native successful mutation arrays are modeled as
`{board_id, domain, action, domains}`, where `domain` is the submitted value
and `action` is `add` or `remove`. They describe the returned snapshot;
they do not claim that a requested addition/removal changed sharing.
Unknown properties and missing or incorrectly typed domain fields cause a
protocol error.

All five commands support the global `--server`, `--profile`, `--output`
(`human` or `json`), `--allow-insecure-http`, and `--help` options.

## Attachment links

```console
wekan board attachment list --board BOARD_ID
wekan --output json board attachment list --board BOARD_ID
```

This command calls `GET /api/boards/{boardId}/attachments` with the selected
profile's stored credential. It returns the native legacy link projection,
including board backgrounds. The required `--board` ID must be nonblank and
cannot be `.` or `..`; it is encoded as one path segment.

Success data contains `board_id`, a derived `count`, and `attachments`. Each
entry maps `attachment_id`, `attachment_name`, `attachment_type`, `url`,
`url_download`, `board_id`, `swimlane_id`, `list_id`, and `card_id`. Optional
properties omitted by Wekan appear as null in CLI output; board backgrounds
omit swimlane/list/card selectors. Native URLs are preserved unchanged and are
not fetched. Both human and JSON output include all mapped fields; human output
escapes terminal controls. Unknown properties and invalid known fields fail
strict decoding.

Wekan requires an authenticated board member with the permissions accepted by
its `checkBoardAccess` helper, or a site administrator. Nonmembers and workers
receive HTTP 403; absent/invalid authentication receives HTTP 401, and a missing
board receives HTTP 404. The pinned route sends these errors as HTML; normal CLI
error mapping retains their HTTP status.

Supported global options are `--server <URL>`, `--profile <NAME>`,
`--output human|json`, `--allow-insecure-http`, and `--help`. Flags do not repeat.
For modern size/storage metadata and transfers, use the
[attachment family](attachments.md).
