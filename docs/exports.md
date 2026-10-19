# Native exports

Export artifacts with the resource that owns them:

```console
wekan board export BOARD --format json --destination board.json
wekan board export BOARD --format xlsx --destination board.xlsx --auth-token-query
wekan card export CARD --board BOARD --list LIST --format pdf --destination card.pdf --auth-token-query
wekan attachment export ATTACHMENT --board BOARD --destination attachment.json --auth-token-query
wekan --output raw board export BOARD --format csv --destination - --no-auth
```

Board formats are `json`, `csv`, `tsv`, `xlsx`, `pdf`, `zip`, `kanboard`, `deck`,
`openproject`, `github`, `gitlab`, `gitea`, `forgejo`, `trello`, `jira`, `asana`, and
`zenkit`. Card formats are `xlsx` and `pdf`. Attachment export produces Wekan's
native JSON object containing metadata and nullable base64 file data; use
`attachment download` for the decoded file. Formats are required for board/card
exports and are never inferred from the destination extension.

These commands call the ten native export operations. They make one request,
without resource lookups, retries, authentication fallback, or export emulation.
The server determines which information each format contains. External-tool JSON
exports are Wekan's format projections, not a complete backup.
The default detailed XLSX is a report. Excel import interprets its first-sheet
rows as card data and can create extra cards; see [import behavior](imports.md#new-boards).
Trello exports preserve null entries in card label ID arrays, including those
stored by `card update --labels '[null]'`.

## Authentication and timeouts

The default sends the selected profile's stored credential as a Bearer token.
`--auth-token-query` instead sends that token as Wekan's `authToken` query
parameter. `--no-auth` skips credential access and sends no token, for public
exports. These two flags conflict. All existing profile, server, output, and
insecure-HTTP global options remain available.

Private board CSV/TSV, board/card Excel, and attachment JSON routes have
administrator-only Bearer checks in v11.06. A normal board member can use their
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

Board exports additionally accept `board-header`, `activities`, and `card-details`.
`card-details` selects the card-style board Excel layout; an explicit selection
without it selects the legacy table workbook. Kanboard has no field selection.
The CLI validates section names, while Wekan decides which sections affect each
format. External JSON formats gate description, labels, and dates where those
exist. Private CSV applies the selection to columns; public CSV ignores it.

Board JSON/XLSX/PDF/ZIP support `--swimlane ID`, `--list ID`, `--card ID`, and
`--checklist ID`. Scope IDs must contain 1–64 ASCII letters, digits, underscores,
or hyphens. Supplied selectors are forwarded together: card wins over checklist;
otherwise list and swimlane constrain the cards together. A checklist scope
selects its parent card. Scope flags are unavailable for the other formats.

Board JSON also accepts `--attachments true|false`; false omits base64 file data,
without dropping attachment metadata when the attachments section is selected.
CSV accepts `--delimiter TEXT`; TSV uses the CSV endpoint with a tab and does not
accept a delimiter override.

Board/card XLSX and PDF accept `--language CODE`, `--date-format FORMAT`, and
`--timezone ZONE`. Date formats are `YYYY-MM-DD`, `DD-MM-YYYY`, or `MM-DD-YYYY`;
timezone strings may contain at most 64 UTF-16 code units. Empty means UTC.
Wekan can prefer the saved profile language and applies its native language
fallback. Public card Excel ignores date format and timezone in v11.06.
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

Successful file exports return `resource` (`board`, `card`, or `attachment`),
`board_id`, nullable `list_id`, `card_id`, and `attachment_id`, `format`,
`destination`, `bytes_written`, `file_saved: true`, `http_status`, and
`content_type`. JSON output wraps this metadata in the normal success envelope;
the artifact contents and staging path are excluded. Human output escapes terminal
control characters. If completion output fails after saving, error details retain
`file_saved: true` and `destination`.

## Pinned Wekan v11.06 behavior

- ZIP export fails with HTTP 500 before producing an archive (W11-005). The CLI
  exposes this native failure and does not build a replacement ZIP.
- Several Excel/PDF failure branches return plain text with HTTP 200, sometimes
  without Content-Type (W11-008). They produce `protocol_error`, exit 5, retaining
  `http_status: 200` and the safe native message. No artifact is delivered.
- Card Excel with a recognized fields selection that omits `board-info` fails
  with HTTP 500. Include `board-info` explicitly when that content is wanted;
  the CLI never inserts it automatically.
- A missing attachment ID exports `{}` successfully. A missing board still uses
  the route's native failure behavior.

Other invalid response shapes/media/signatures use `protocol_error`, exit 4.
Attachment JSON can contain `file: null`, including for GridFS-only files;
successful file delivery does not establish that attachment bytes were included.
File staging/read/save failures use `file_io_error`, exit 4. Native non-200
responses use the existing server, permission, authentication, and not-found
classifications and retain their actual HTTP status.
