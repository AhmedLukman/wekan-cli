# Native exports

Native JSON export validation includes milestone and shared-label metadata,
group and custom-role memberships, and checklist item assignee and linked-card
references. Their original field names and omission are preserved in the artifact.
Custom-field records preserve `readOnly`, `adminOnly`, `cardKinds`, and `sort`;
movement activities preserve `moveReason`, including an empty reason. These
properties may be absent on older records, and unrelated activities omit the
movement reason. Present values must have their native types; unknown fields
continue to fail validation.

Export artifacts with the resource that owns them:

```console
wekan board export BOARD --format json --destination board.json
wekan board export BOARD --format xlsx --destination board.xlsx --auth-token-query
wekan card export CARD --board BOARD --list LIST --format pdf --destination card.pdf --auth-token-query
wekan attachment export ATTACHMENT --board BOARD --destination attachment.json --auth-token-query
wekan --output raw board export BOARD --format csv --destination - --no-auth
```

All 48 board formats are supported:

```text
json csv scsv tsv xlsx pdf zip ics
kanboard deck openproject github gitlab gitea forgejo trello
jira asana zenkit markdown leo todotxt taskwarrior focalboard
todoist planner meistertask obsidian linear ticktick clickup nullboard
kanri pivotal redmine tasksorg monday businessmap superproductivity taiga
vikunja wrike wrikeworkflow teamwork quire notion opml orgmode
```

Card formats are `xlsx` and `pdf`. Attachment export produces Wekan's
native JSON object containing metadata and nullable base64 file data; use
`attachment download` for the decoded file. Formats are required for board/card
exports and are never inferred from the destination extension.

These commands call the corresponding native export operations. They make one request,
without resource lookups, retries, authentication fallback, or export emulation.
The server determines which information each format contains. External-tool JSON
exports are Wekan's format projections, not a complete backup.
The default detailed XLSX is a report. Excel import interprets its first-sheet
rows as card data and can create extra cards; see [import behavior](imports.md#new-boards).
Trello exports preserve null entries in card label ID arrays, including those
stored by `card update --labels '[null]'`.
GitLab exports preserve release milestones and sprint iterations, with planned
dates omitted when unset. Retained comments whose authors were deleted preserve
the native empty `author` object. Kanboard, Deck, OpenProject, Jira, Asana, and
Zenkit also retain comments whose authors were deleted, preserving omitted
identity properties and any remaining empty author objects.

OpenProject exports preserve release version links and sprint goals as
descriptions. Release descriptions and release/sprint planned dates may be
omitted when unset. Present properties remain typed, and unknown properties
still fail validation.

## Authentication and timeouts

The default sends the selected profile's stored credential as a Bearer token.
`--auth-token-query` instead sends that token as Wekan's `authToken` query
parameter. `--no-auth` skips credential access and sends no token, for public
exports. These two flags conflict. All existing profile, server, output, and
insecure-HTTP global options remain available.

Private board CSV/TSV, board/card Excel, and attachment JSON routes have
administrator-only Bearer checks in v12.57. A normal board member can use their
own credential with the explicit `--auth-token-query` option. The CLI does not
switch modes after a rejection. Other routes retain their own native permission
checks.

`--timeout SECONDS` overrides the 30-second total HTTP request timeout with a
positive integer. Validation and file delivery happen after the network response
has been received. Tokens are omitted from diagnostics and success metadata.

## Selecting content

Repeat `--field SECTION` to send one native comma-separated fields selection.
Omitting it requests Wekan's default selection. Card sections are:

```text
labels people board-info dates description custom-fields checklists subtasks
comments attachments voting poker stickers locations dependencies sort
```

Board exports additionally accept `board-header`, `activities`, `card-details`, and `scrum`.
`card-details` selects the card-style board Excel layout; an explicit selection
without it selects the legacy table workbook. Kanboard has no field selection.
The CLI validates section names, while Wekan decides which sections affect each
format. External JSON formats gate description, labels, and dates where those
exist. Private CSV applies the selection to columns; public CSV ignores it.

Board JSON/XLSX/PDF/ZIP/ICS support `--swimlane ID`, `--list ID`, `--card ID`, and
`--checklist ID`. Scope IDs must contain 1–64 ASCII letters, digits, underscores,
or hyphens. Supplied selectors are forwarded together: card wins over checklist;
otherwise list and swimlane constrain the cards together. A checklist scope
selects its parent card. Scope flags are unavailable for the other formats.

Board JSON also accepts `--attachments true|false`; false omits base64 file data,
without dropping attachment metadata when the attachments section is selected.
CSV accepts `--delimiter TEXT`; TSV uses the CSV endpoint with a tab and does not
accept a delimiter override.

Board/card XLSX and PDF accept `--language CODE`, `--date-format FORMAT`, and
`--timezone ZONE`. Date formats are `YYYY-MM-DD`, `DD-MM-YYYY`, or `MM-DD-YYYY`, including
the corresponding `-date-only` variants;
timezone strings may contain at most 64 UTF-16 code units. Empty means UTC.
Wekan can prefer the saved profile language and applies its native language
fallback. Native card-style Excel output stores dates as Excel date cells in
UTC with `yyyy-mm-dd hh:mm` formatting in both public and private workbooks;
those cells retain native spreadsheet values rather than formatted date text.
Unsupported format/option combinations fail with exit 2 before target or
credential access or HTTP.

## File delivery and output

`--destination PATH` is required. Exports stream to a temporary file beside the
destination, then undergo validation before being published atomically. Existing
files are protected unless `--overwrite` is supplied. Parent directories must
already exist. Staging files are cleaned up on failure.

`--destination -` requires `--output raw` and stages in the system temporary
directory. It emits the validated original bytes without an envelope, added
newline, or status text. File destinations require `--output human` or `json`.
`--overwrite` conflicts with stdout. Shell redirection can truncate a file before
the CLI starts; use a file destination for replacement protection.

There is no aggregate 64 MiB export limit. The CLI needs disk space for the entire
artifact. JSON is validated one array element at a time; validation memory depends
on the current record/string rather than the total export. Large individual
attachment strings can still require substantial memory. JSON objects, including
activity and rule payloads, reject unmapped fields, invalid types, and missing
required values. Custom schemaless payload properties can therefore cause
`protocol_error`; they are never silently discarded. The native bytes are
preserved after validation, including import activities' `source` metadata:
`system`, optional `id` (omitted on native Wekan reimports without an `id`), and
optional Trello `url`. Native checklist records preserve the optional `userId`
creator field: UI/DDP creation supplies it, while REST creation omits it.
Card and subtask records preserve optional `watchers` user ID arrays. Untouched
cards omit the field; removing the last watcher leaves an empty array.
Binary validation checks the expected content type and file signature; it does
not prove that every page/workbook entry is intact.

Successful file exports return `resource` (`board`, `boards`, `chart`, `card`, or `attachment`),
`board_id`, nullable `chart_key`, `list_id`, `card_id`, and `attachment_id`, `format`,
`destination`, `bytes_written`, `file_saved: true`, `http_status`, and
`content_type`. JSON output wraps this metadata in the normal success envelope;
the artifact contents and staging path are excluded. Human output escapes terminal
control characters. If completion output fails after saving, error details retain
`file_saved: true` and `destination`.

## Pinned Wekan v12.57 behavior

- Native ZIP export now produces an archive; the old CommonJS interop failure is fixed.
- Some Excel/PDF failures still return plain text with HTTP 200 (W11-008).
  The CLI reports `protocol_error` and does not deliver an invalid artifact.
- Card Excel with a recognized fields selection that omits `board-info` fails
  after the response starts and aborts the transfer (W11-028). The CLI reports
  `transport_error`. Include `board-info` explicitly when that content is wanted;
  the CLI never inserts it automatically.
- A missing attachment ID exports `{}` successfully. A missing board still uses
  the route's native failure behavior.

Other invalid response shapes/media/signatures use `protocol_error`, exit 4.
GridFS-only attachment JSON contains `file: null`, and board JSON contains an
empty `file` string despite successful binary downloads (W11-063);
successful file delivery does not establish that attachment bytes were included.
File staging/read/save failures use `file_io_error`, exit 4. Native non-200
responses use the existing server, permission, authentication, and not-found
classifications and retain their actual HTTP status.


## Calendar, reports, and multiple boards

```console
wekan board export BOARD --format ics --destination calendar.ics
wekan board chart export burndown --board BOARD --format pdf --destination burndown.pdf
wekan board chart export scrumSprint --board BOARD --sprint SPRINT --format xlsx --destination sprint.xlsx
wekan board export-all --format wekan --board BOARD --destination boards.zip
```

Calendar exports support the board scope selectors and retain native VEVENT
content. Chart exports support `pdf|xlsx` and these 21 chart keys:

```text
dashboard burndown burnup cumulativeFlow controlChart cycleTime flowEfficiency
leadTime throughputHistogram wipRun gantt time pulse scrumVelocity scrumSprint
scrumDaily agingWip blockerAnalysis monteCarlo processBehavior sizeCycleTime
```

`scrumSprint` and `scrumDaily` require `--sprint`. Report controls are
`--target-count 1..10000`, `--history-days 7..365`, `--target-date YYYY-MM-DD`,
and `--size-field poker|CUSTOM_FIELD_ID`. Language, date-format, timezone,
authentication, timeout, and artifact-delivery options follow the other exports.

`board export-all` accepts `wekan`, `csv`, `scsv`, `tsv`, `excel`, and every
external board format listed above. Repeat `--board` to select boards; omission
uses all eligible boards. Repeat `--field` to select sections and use
`--attachments true|false` to control bytes where supported. Excel produces one
workbook with a sheet per board; other formats produce ZIPs. Wekan records
skipped boards and errors in `skipped.txt`, which must be inspected for partial
outcomes. `--no-auth` is unavailable for mass export.

W12-003: mass CSV/SCSV/TSV currently return a ZIP containing only `skipped.txt`
for an ordinary populated board. Internal validation rejects the server's list
objects and disables the exporting account. W12-002: a normal member's Bearer
attachment JSON export also incorrectly disables that account after HTTP 403.
These are recorded native defects; the CLI performs no authentication fallback
or export emulation.

Native ZIP exports contain independently readable file bytes, including GridFS
files and distinct entries for duplicate filenames. Their filenames omit the
attachment IDs the raw ZIP importer uses, so a direct scoped ZIP round trip can
import the cards while reporting zero attachments (W12-004). New-board ZIP
import currently returns HTTP 500 because its job lacks the user context
(W12-005). Both behaviors are regression-tested without altering the archive.
