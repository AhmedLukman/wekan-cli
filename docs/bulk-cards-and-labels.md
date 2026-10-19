# Bulk cards and labels

These commands use Wekan v11.06's native batch endpoints. Each batch sends one
request containing 1–500 cards; larger inputs fail locally. All commands support
the standard global options and human/JSON output.

## Create cards

```console
wekan card bulk create --board BOARD --list LIST --swimlane-id SWIMLANE --cards-file cards.json
wekan card bulk create --board BOARD --list LIST --cards-stdin
```

Supply exactly one of `--cards <JSON>`, `--cards-file <PATH>`, or `--cards-stdin`.
All three sources have a 50 MiB client JSON input limit. File sources must be
regular files; use `--cards-stdin` for streams. Oversized inputs fail before HTTP.
Files and stdin contain a UTF-8 JSON array, not a request wrapper:

```json
[
  {"title":"First card","description":"Details","members":["USER_ID"]},
  {"title":null,"swimlaneId":"OTHER_SWIMLANE","parentId":"PARENT_CARD_ID"}
]
```

Supported properties are `title`, `description`, `parentId`, `swimlaneId`,
`members`, and `assignees`. Unknown properties and invalid field types fail
before HTTP. Omitted title defaults to an empty string; explicit null titles
and descriptions remain distinct from omission. The optional `--swimlane-id`
sets a default; each card's nonempty `swimlaneId` takes precedence. Wekan accepts
`members` and `assignees` as JSON values, filters array contents to nonempty
strings, and retains only active board members. Values are sent unchanged.
Author attribution always uses the authenticated session.

Each card needs an effective swimlane. Wekan v11.06 responds with HTTP 500 if
one is absent, potentially after earlier cards were created. The CLI preserves
that failure and its HTTP status; inspect stored cards before retrying.
No remote validation or repair precedes the request.

## Discover and add labels

```console
wekan board label list --board BOARD
wekan board label add --board BOARD --name Bug --color red
wekan card label list --board BOARD --list LIST --card CARD
```

Board label creation requires board-administrator or site-administrator access.
`--name` is optional; omission produces an empty name. `--color` defaults to
green. Explicit colors must be named values shown by `board label add --help`;
the REST route rejects custom hex colors despite the stored-label schema
supporting them.

Names are sent unchanged. Wekan trims JavaScript whitespace on storage but
checks for an existing name/color pair before cleaning the name. Consequently,
repeating a padded name creates another label; sending the cleaned name reuses
the existing label. The CLI preserves this pinned defect. Output reports an
added-or-reused acknowledgment with `board_id` and `label_id`.

Board label listing preserves stored order and returns IDs, nullable names, and
colors. Card label listing returns stored IDs and preserves null elements. Both
strictly decode the complete underlying document before projection. Board data
contains `board_id` and `labels` (`label_id`, nullable `name`, `color`); card data
contains `board_id`, `list_id`, `card_id`, and `label_ids`.

## Merge card labels

```console
wekan card bulk labels --board BOARD --card FIRST --card SECOND --add-label BUG --remove-label OLD
```

Repeat `--card`, `--add-label`, and `--remove-label` as needed. At least one label
addition or removal is required. Wekan keeps existing labels minus removals,
then appends additions and removes duplicates. A label present in both changes
is added back. Added labels must exist on the selected board; unknown removed
labels are harmless. Invalid additions return HTTP 400 and `invalid_label_ids`.
Existing null label IDs are preserved in stored arrays and returned `label_ids`;
the merge deduplicates them along with string IDs.

Only active cards on that board are updated. Archived, foreign, and absent cards
are returned in `not_found`. Repeated requested IDs produce repeated update
records in caller order. Normal board members can apply existing labels.

## Delete cards

```console
wekan card bulk delete --board BOARD --card FIRST --card SECOND --yes
```

Deletion requires interactive confirmation or command-local `--yes`; JSON mode
requires `--yes`. Declining confirmation returns an exit-0 cancellation with
operation `card_bulk_delete`. Confirmation identifies the board and requested
card IDs and holds the existing credential mutation guard.
After confirmation, the CLI reloads the guarded credential and checks its
identity and expiry. A replaced, removed, or expired credential prevents HTTP
and leaves the current stored credential untouched.

Wekan deletes matching cards, including archived cards, and cascades their
comments, checklists, checklist items, and subcards. Wekan v11.06 has an
attachment-cascade defect: ordinary attachment metadata and file bytes remain
after successful card deletion, and board attachment listing still returns them.
The `deleted` array confirms card deletion, not attachment cleanup. The CLI
preserves this behavior without additional cleanup requests.
Foreign and absent IDs are returned in `not_found`. Duplicates are sent unchanged,
so a later occurrence can be unmatched after an earlier deletion.

## Batch results and automation

Fully successful requests return the standard `ok: true` envelope and exit 0.
Any creation item error or nonempty `not_found` produces `batch_incomplete` and
exit 5. Its error details retain HTTP 200 and the entire typed result:

```json
{"ok":false,"error":{"code":"batch_incomplete","message":"...","details":{"batch_result":{"operation":"card_bulk_delete","data":{"board_id":"BOARD","deleted":["FIRST"],"not_found":["SECOND"]}},"http_status":200,"http_success":true,"retry_safe":false}}}
```

Errors go to stderr with empty stdout, including all returned successes.
Creation data contains `board_id`, `list_id`, and `results` with zero-based
`index`, nullable `card_id`, and nullable `error`. Deletion data contains
`board_id`, `deleted`, and `not_found`. Label results contain `updated` objects with `card_id` and
`label_ids`, plus `not_found`. No response arrays are sorted or deduplicated.
Human output includes counts and every returned outcome. Unknown response
fields, including nested fields, are protocol errors.

The CLI performs no automatic retries, rollback, label editing/deletion
emulation, or batch chunking. Request-level failures can follow partial writes.
Single-card replacement remains available through `card update --label`,
`--labels`, or `--clear-labels`.

File/stdin read failures use `file_io_error`, exit 4. Invalid JSON, unsupported
properties, invalid field types, and batch sizes use `invalid_input`, exit 2.
