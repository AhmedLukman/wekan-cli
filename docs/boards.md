# Boards

The authenticated `wekan board` family implements the core Wekan v12.57 board
lifecycle:

```text
wekan board list [--public]
wekan board count
wekan board get <BOARD_ID>
wekan board create --title <TITLE> [--permission private|public|instance] [--color <COLOR>]
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

`board count` returns Wekan's server-wide private, public, and instance counts. These are
not counts of boards visible to the authenticated caller.

## Creating and renaming

`--title` must be non-empty. The authenticated caller is always the initial
active board administrator. Permission defaults to `private`, and color defaults to
`belize`. Accepted v12.57 colors are `belize`, `nephritis`, `pomegranate`,
`pumpkin`, `wisteria`, `moderatepink`, `strongcyan`, `limegreen`, `midnight`,
`dark`, `relax`, `corteza`, `appleglasspastel`, `clearblue`, `cleargreen`,
`clearorange`, `clearpink`, `clearpurple`, `clearred`, `natural`, `modern`,
`moderndark`, `exodark`, `cleandark`, and `cleanlight`.

The former creation options `--owner`, `--is-admin`, `--is-active`,
`--no-comments`, `--comment-only`, and `--worker` have been removed because
v12.57 ignores these inputs. Change membership through `board member` commands
after creation. `rename` requires a non-empty title and verifies both the ID
and title returned by Wekan.

## Board detail

`board get` returns the complete normalized v12.57 board model. Names are
snake_case, `_id` becomes `board_id`, `backgroundImageURL` becomes
`background_image_url`, and the model's `type` becomes `board_type`. It includes
identity, timestamps, ordering, typed members, sharing records, labels,
watcher records, organization/team/domain records, theme/background values, date defaults,
card-display controls, and board settings.

Labels preserve optional `due_at` milestone timestamps and `shared_label_id`.
Members preserve optional `granted_by` group identifiers and `custom_role_id`.
These nested fields are null in stable output when omitted by Wekan; native
JSON exports preserve their original names and presence.

Known enum fields accept only their corrected v12.57 values, and timestamp
fields must contain RFC 3339 date-times. Other values cause a protocol error.

Known omitted scalar fields are JSON `null`. Omitted `labels`, `watchers`,
`orgs`, `teams`, `domains`, `import_usernames`, and `custom_theme_colors` become
empty arrays; explicit nulls for these collections are rejected. Optional
metadata collections, such as `column_sets` and `wip_limit_groups`, are null
when absent. `members` is required and is never normalized from null or
omission. The board ID and title must be non-empty.
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
{"private":2,"public":1,"instance":0}
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

All 111 boolean flags accept `<true|false>`:

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
- `--allows-swimlane-dragging`
- `--allows-list-dragging`
- `--allows-card-dragging`
- `--allows-checklist-dragging`
- `--allows-checklist-item-dragging`
- `--allows-subtask-dragging`
- `--allows-minicard-collapse`
- `--labels-above-title-on-minicard`
- `--allows-checklist-due-date`
- `--allows-checklist-title`
- `--ask-for-move-reason`
- `--allows-comments-on-minicard`
- `--allows-spent-time`
- `--allows-spent-time-on-minicard`
- `--allows-swimlane-name-on-minicard`
- `--show-label-text`
- `--allows-stickers`
- `--allows-stickers-on-minicard`
- `--allows-location`
- `--allows-dependencies`
- `--allows-dependencies-on-minicard`
- `--allows-flowtime`
- `--allows-pomodoro`
- `--allows-vote`
- `--allows-vote-on-minicard`
- `--allows-poker`
- `--allows-poker-on-minicard`
- `--allows-text-notes`
- `--allows-comment-count-on-minicard`
- `--allows-issue-type-on-minicard`
- `--allows-card-color-on-card`
- `--allows-card-color-on-minicard`
- `--allows-parent-card-on-card`
- `--allows-parent-card-on-minicard`
- `--allows-linked-card-on-card`
- `--allows-linked-card-on-minicard`
- `--allows-card-buttons-on-card`
- `--allows-description-badge-on-minicard`
- `--allows-unread-comments-on-minicard`
- `--allows-card-collapse`
- `--allows-checklist-title-on-minicard`
- `--allows-label-text-on-card`
- `--allows-card-buttons-on-minicard`
- `--allows-description-badge-on-card`
- `--allows-unread-comments-on-card`
- `--allows-due-from-title-on-card`
- `--allows-due-from-title-on-minicard`
- `--allows-similar-cards-on-card`
- `--allows-similar-cards-on-minicard`
- `--allows-at-risk-on-card`
- `--allows-at-risk-on-minicard`
- `--allows-suggested-members-on-card`
- `--allows-forecast-on-card`
- `--allows-forecast-on-minicard`
- `--allows-checklist-from-description-on-card`
- `--allows-action-items-on-card`
- `--allows-whiteboard-on-card`
- `--labels-above-title-on-card`
- `--allows-checklist-due-date-on-minicard`
- `--allows-location-on-minicard`
- `--allows-swimlane-name-on-card`
- `--allows-flowtime-on-minicard`
- `--allows-pomodoro-on-minicard`
- `--allows-attachment-list-on-minicard`
- `--allows-text-notes-on-minicard`
- `--allows-comment-count-on-card`
- `--allows-activities-on-minicard`

`--card-aging-days1`, `--card-aging-days2`, and `--card-aging-days3` accept
integer days from 0 through 9007199254740991 (JavaScript's maximum safe integer).
Negative, fractional, and larger inputs fail locally. Threshold order is not
restricted. Flags cannot repeat. Use `wekan api request` for Wekan's broader
string/JSON input coercions.

Both reads and updates return `board_id` and a complete `settings` object with
all 111 boolean settings and three aging thresholds, including optional flags
set through Wekan's board settings interface. Updates also return
`submitted_fields` in corrected-contract order. Settings
keys match the snake_case names in `board get`. Omitted response properties
appear as JSON null; explicit null, incorrect types, and unknown properties
are protocol errors. Stored numeric thresholds can be fractional or negative
and are returned without coercion. Human output includes every setting.

Reading uses Wekan's board-access check; updating requires board administration.
Normal and read-only members can read settings, while board administrators and
site administrators can update them. Permission and missing-board failures
preserve Wekan's HTTP classification.

## Domain sharing

```console
wekan board domain list --board BOARD_ID
wekan board domain add example.com --board BOARD_ID
wekan board domain remove example.com --board BOARD_ID
```

Domain listing uses the dedicated endpoint and requires a **site administrator**,
including for a public board or a board administered by the caller. It includes
inactive records in stored order. Regular callers receive HTTP 403, mapped to
`permission_denied`.

Addition sends the domain unchanged as JSON. Removal encodes the supplied value
as one path segment and rejects empty values and the exact values `.` and `..`
before HTTP. Wekan handles trimming, lowercasing, and validation. Removal runs
directly without confirmation or `--yes`.

**Pinned Wekan v12.57 defect:** valid board/site-administrator domain additions
and removals fail with HTTP 500 because `setBoardDomains`
receives no DDP user identity. Sharing remains unchanged, including when adding
an existing domain or removing an absent one. The CLI returns `server_error`
with exit 5, `http_status: 500`, and `server_error: "Internal server error"`.
Wekan does not expose the underlying authorization error or its metadata.
It sends the native request without a workaround or retry.
See the [v12.57 route and method implementation](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js)
and [public error mapping](https://github.com/wekan/wekan/blob/v12.57/server/lib/apiResponseHelpers.js).

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

## View settings and card section order

```console
wekan board view-settings get --board BOARD_ID
wekan board view-settings update --board BOARD_ID --default-private-view board-view-swimlanes --view-order board-view-swimlanes --view-order board-view-cal
wekan board view-settings update --board BOARD_ID --view-settings '{"board-view-cal":{"showOnPublic":false,"showOnPrivate":true}}'
wekan board card-field-order get --board BOARD_ID
wekan board card-field-order update --board BOARD_ID --field description --field labels --field dates
wekan board card-field-order update --board BOARD_ID --reset
```

View updates require at least one of `--view-settings`, `--default-public-view`,
`--default-private-view`, or repeated `--view-order`. Card order updates require
repeated `--field` values or `--reset`; these forms conflict. Reads return the
server's normalized settings and available section keys. Updates use native
board administrator permissions and preserve Wekan's ordering rules.
