# Imports

Import commands use Wekan v11.06's native REST endpoints. They accept ordinary
files or `-` for stdin, require stored credentials, and run directly without a
confirmation prompt. Use `--output json` for stable machine-readable results.
They do not retry requests: repeated imports create additional resources.

## New boards

```console
wekan board import board.json
wekan board import trello.json --source trello
wekan board import cards.csv --source csv --delimiter ";"
wekan board import cards.tsv --source csv --delimiter '\t'
wekan board import workbook.xlsx --source excel
wekan board import issues.json --source github --members-mapping-file members.json
```

Omitting `--source` selects `POST /api/boards/import`. An explicit source selects
`POST /api/boards/import/{source}`, including `--source wekan`. Supported sources
are `wekan`, `trello`, `csv`, `jira`, `kanboard`, `excel`, `deck`, `openproject`,
`github`, `gitlab`, `gitea`, `forgejo`, `asana`, and `zenkit`.

Format is explicit; filename extensions have no effect. JSON sources accept an
export document (object or array). The CLI preserves its properties and wraps
the entire document in the request's `board` property. Supply the export itself,
including Kanboard's own `board` header, rather than an API request wrapper.
Wekan interprets the platform's document and validates its contents.

CSV input is UTF-8 text with a header row. The CLI sends all records as arrays
of strings, retaining the header, quoted delimiters, multiline fields, field
whitespace, empty fields, record order, and variable record lengths. Empty
physical lines are skipped by the CSV reader. Wekan recognizes headers such as
`title`, `description`, `stage`, `members`, and `due date`; it interprets dates
and members. `--delimiter` is CSV-only, defaults to comma, and accepts one ASCII
character other than NUL, CR, LF, or double quote, or the literal `\t` for tab.
Excel input is XLSX bytes, encoded as `excelBase64`; Wekan reads its first sheet.

Optional member mapping is an object of source identifiers to local Wekan user
IDs, for example `{"upstream-alice":"local-user-id"}`. Use either
`--members-mapping <JSON>` or `--members-mapping-file <PATH>`. The mapping file
must be a regular UTF-8 JSON file; `-` is reserved for the main document input.
Mapping behavior belongs to the selected importer. For example, CSV adds mapped
card members and omits unmapped ones. This command performs no account lookup
or reconciliation.

The board result contains `board_id`, `source` (`native` when omitted), and
`mutation_confirmed: false`.

## ZIP into an existing board

```console
wekan board import-zip board.zip --board BOARD_ID
wekan board import-zip board.zip --board BOARD_ID --list LIST_ID --field description --field comments
wekan board import-zip board.zip --board BOARD_ID --card CARD_ID --timeout 120
```

The complete ZIP is sent as `application/zip`, containing `wekan.json` and
optional `attachments/<id>-<filename>` entries. Files are streamed from an open
regular-file handle; stdin is staged on disk before the request and removed
afterwards. Archives are not extracted or rewritten by the CLI.

Optional `--swimlane`, `--list`, `--card`, and `--checklist` selectors are sent
together as supplied. Scope IDs require 1–64 ASCII letters, digits, underscores,
or hyphens. Native card scope takes precedence over list and swimlane scope.
**Wekan v11.06 ignores checklist scope**, so `--checklist` alone can still import
the whole document into the destination board. No client emulation occurs.

Repeat `--field` for selected sections: `card-details`, `board-header`,
`activities`, `labels`, `people`, `board-info`, `dates`, `description`,
`custom-fields`, `checklists`, `subtasks`, `comments`, `attachments`, `voting`,
`poker`, `stickers`, `locations`, `dependencies`, or `sort`. Values are joined
into one native `fields` parameter; Wekan decides each section's import effect.
Omission leaves native selection defaults.

Bearer authentication is the default. `--auth-token-query` explicitly sends the
stored token through `authToken` instead. There is no authentication fallback.

The ZIP result includes `board_id`, requested `swimlane_id`, `list_id`, `card_id`,
and `checklist_id` (null when absent), `counts`, and `mutation_confirmed: false`.
The six counts are `swimlanes`, `lists`, `cards`, `checklists`, `comments`, and
`attachments`. Requested selectors do not prove that Wekan honored them.

## Calendar events as cards

```console
wekan card import-ics events.ics --board BOARD_ID --swimlane-id SWIMLANE_ID --list LIST_ID
```

UTF-8 calendar contents are sent in the native `ics` JSON property. Wekan creates
cards from VEVENTs, using DTSTART for `startAt` and DTEND for `dueAt`, or DTSTART
when DTEND is absent. The CLI does not parse, normalize, or synchronize events.

The result contains `board_id`, `swimlane_id`, `list_id`, `created`, `card_ids`,
and `mutation_confirmed: false`. Human output includes the returned count and
card IDs. All import results describe server acknowledgements without readback.

## Limits and errors

JSON, CSV, XLSX, ICS, and member mapping inputs each have a 50 MiB acquisition
limit. The final serialized JSON request also has a 50 MiB limit, including
wrapping, escaping, member mapping, and base64 expansion. Consequently the
largest accepted XLSX is smaller than 50 MiB. ZIP input has a 5 GiB client limit;
the server may impose a lower configured limit. Invalid local input fails with
`invalid_input` (exit 2); acquisition/staging I/O errors use `file_io_error`
(exit 4). Regular files and stdin are supported; directories and named pipes
are rejected as file inputs.

Every command accepts `--timeout <SECONDS>` with a positive integer; default
30 seconds. All existing global server/profile/output/network options apply.
Raw output and `--yes` are unsupported.

Wekan's HTTP-200 embedded import errors remain errors. A native exception may
serialize as `{}`; strict decoding reports `protocol_error` (exit 4) rather than
claiming success. Unknown response fields, including nested ZIP counters, are
also protocol errors. The existing error envelope preserves native HTTP/error
metadata and reports `outcome_unknown: true` when mutation effects are uncertain.
Missing ZIP boards retain HTTP 404. ICS missing boards or insufficient write
permission return `permission_denied`, retaining HTTP 401 and `not-authorized`
without advising credential replacement; missing lists/swimlanes on an accessible
board retain HTTP 500 HTML errors.

**ZIP attachment bytes can hang after earlier resources have been written.**
The pinned Meteor-Files call receives an invalid callback argument and never
settles. A client timeout does not undo those writes or clean the server's
temporary files. The CLI exposes the timeout and uncertain outcome without a
retry, preflight, rollback, attachment omission, or replacement implementation.
