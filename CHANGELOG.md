# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `user token create` and `user token delete` expose both native API token
  operations, with secret delivery and explicit unconfirmed status on creation,
  stdin input for single-token deletion, explicit delete-all confirmation, and
  human/JSON output. Wekan
  v11.06's unconfirmed single-token matching and missing-user issuance behavior
  are preserved and documented. Self-targeted delete-all rejects changed
  credentials and serializes concurrent local logout/login through its request.

## [0.19.0] - 2026-10-17

### Added

- `board import`, `board import-zip`, and `card import-ics` expose all four native
  import operations with file/stdin input, member mapping, CSV and XLSX encoding,
  ZIP scope/section controls, timeout overrides, and typed human/JSON results.
  Pinned HTTP-200 errors, ignored ZIP checklist scope, and attachment timeouts
  remain explicit failures or documented server behavior. ICS destination/access
  rejections report permission errors while retaining native HTTP metadata.

## [0.18.0] - 2026-10-16

### Added

- `board export`, `card export`, and `attachment export` for all ten native export
  operations, with format, scope, section, locale, and explicit authentication
  controls. Pinned ZIP and Excel/PDF failures remain explicit errors.
- Export files use protected atomic destinations or validated raw stdout, with
  disk staging, strict JSON decoding, and no aggregate 64 MiB export limit.
  Native board JSON includes import history and its source metadata.

### Fixed

- Board JSON exports accept watched cards and subtasks, preserving watcher IDs
  and empty watcher arrays after unwatching.
- Board JSON exports accept UI-created checklists and preserve their optional
  creator IDs.
- Trello board exports preserve null card label IDs instead of failing with a
  protocol error.

## [0.17.0] - 2026-10-16

### Changed

- **Breaking:** Rename `checklist-item` commands to `checklist item` and
  `custom-field-item` commands to `custom-field item`. The old spellings are no
  longer accepted; arguments and structured output are unchanged.

## [0.16.0] - 2026-10-15

### Added

- `admin domain list`, `admin org list`, and `admin team list` for domain user
  counts and organization/team feature summaries.
- Individual and instance-wide organization/team feature updates, with typed
  booleans, complete human/JSON results, and confirmation for bulk changes.

## [0.15.0] - 2026-10-15

### Added

- `admin settings get/update` and `admin attachment-settings get/update` for
  global REST settings, storage backends, upload restrictions, transfer limits,
  and migration configuration, with typed flags and strict JSON file/stdin input.
- Cloud credential file/stdin inputs, masked credential-presence output, complete
  human/JSON settings, and explicit errors for unmapped response fields.

## [0.14.0] - 2026-10-15

### Added

- `rule list/get/create/update/delete` for board automation, with bounded typed
  JSON definitions from arguments, files, or stdin and complete human/JSON output.
- Rule payloads cover all built-in activity, schedule, button, and action fields.
  Updates preserve native merge semantics and unconfirmed effects; deletion
  requires confirmation or `--yes` and removes referenced trigger/action documents.

### Fixed

- Rule creation and updates reject invalid `lastRunAt` timestamps before writing
  definitions that rule reads cannot decode.
- Rule get/list accept responses up to 64 MiB so large definitions accepted by
  create/update remain readable.

## [0.13.0] - 2026-10-15

### Added

- `board card-settings get/update` for all card display controls and aging
  thresholds, with explicit boolean/integer inputs and complete typed human/JSON
  output. Updates require at least one setting and preserve omitted fields.
- `board domain list/add/remove` for native domain-sharing operations. Listing
  requires site administration; valid writes preserve Wekan v11.06's
  `not-authorized` failure as exit-5 errors with native metadata.

## [0.12.0] - 2026-10-15

### Added

- `integration list/get/create/update/delete` for board webhooks, including
  stdin token input, explicit token clearing, and complete typed human/JSON output.
- `integration activity add/remove` for webhook subscriptions, plus complete
  replacement or clearing through integration update. Removal runs directly;
  integration deletion requires confirmation or `--yes`.
- Integration update and delete report unconfirmed matched effects. Update errors
  preserve possible partial writes, and missing reads/subscription targets retain
  Wekan v11.06's HTTP 200 metadata in `not_found` errors.

## [0.11.0] - 2026-10-14

### Added

- Native board, card, list, and swimlane copying, plus list/swimlane moving,
  with destination selectors and numeric or exact JSON position inputs.
- `card count` for board/list totals, `card list --swimlane`, and `card get`
  by ID alone, including archived cards. Existing scoped reads remain available.
- All copy/move results report `mutation_confirmed: false` and explain unconfirmed
  effects in human output. Explicit ambiguous-outcome errors preserve
  Wekan v11.06's source-list retention, asynchronous default-swimlane insertion,
  and partial cross-board move failures.
- Copy documentation records Wekan v11.06 returning success while omitting
  card attachments in board, card, list, and swimlane copies.

### Security

- Upgrade the TLS dependency to rustls 0.23.46, including upstream fixes for
  RUSTSEC-2026-0346, RUSTSEC-2026-0347, RUSTSEC-2026-0348, and RUSTSEC-2026-0349.

## [0.10.0] - 2026-10-14

### Added

- Grouped `card bulk create`, `card bulk delete`, and `card bulk labels` commands
  for native batches of up to 500 cards. Creation accepts inline JSON, files,
  or stdin; incomplete batches return exit 5 with every returned outcome.
- `board label add`, `board label list`, and `card label list` for reusable board
  labels and focused reads with typed human/JSON output.

### Fixed

- Bulk card creation bounds JSON input to 50 MiB and rejects non-regular file
  sources before reading, preventing unbounded buffering and pipe hangs.
- Bulk label updates preserve stored null label IDs in successful and incomplete
  results instead of rejecting Wekan's HTTP 200 response.
- Bulk card deletion rechecks stored credentials after confirmation and sends
  no request if the credential was replaced, removed, or expired while waiting.
- Bulk-delete help and confirmation disclose that Wekan v11.06 retains ordinary
  attachment metadata and file bytes even after successful card deletion.

## [0.9.0] - 2026-10-13

### Added

- Nested `board member`, `card member`, and `card assignee` commands for
  membership lists and individual additions/removals, plus board member roles
  and permission updates. Typed human/JSON output distinguishes unconfirmed
  board acknowledgments from returned scoped card snapshots and preserves
  Wekan v11.06 errors and no-op behavior.

## [0.8.0] - 2026-10-13

### Added

- Complete `dependency` commands for board/card listings, create-or-update,
  nullable and empty metadata updates, and confirmed removal. Typed human/JSON
  output preserves Wekan v11.06 defaults, server errors, and unconfirmed
  acknowledgments for updates/removals that may match no edge.

### Fixed

- Card detail reads accept raw dependency type strings, including empty and
  unfamiliar stored values, while retaining strict field and type decoding.

## [0.7.0] - 2026-10-13

### Added

- Complete `attachment` commands for board/scoped listings, metadata, file or
  stdin uploads, file or raw stdout downloads, board backgrounds, copying,
  moving, and confirmed deletion. Downloads preserve existing files unless
  `--overwrite` is supplied and report completed-file context on output errors.
  Strict typed output exposes the board-wide results returned for deeper list
  scopes by Wekan v11.06. Upload, background-upload, copy, and move use 60-second
  deadlines so the pinned server's HTTP 408 failures can be reported.

### Fixed

- Attachment uploads reject named pipes and other non-regular file sources before
  opening them, avoiding FIFO hangs on Unix.
- Attachment deletion rejects credentials removed, replaced, or expired during
  confirmation before sending a destructive request.

## [0.6.0] - 2026-10-13

### Added

- Full `custom-field` and `custom-field-item` commands for definition and dropdown
  management, plus `card set-custom-field` and `card find-by-custom-field`, with
  strict typed human/JSON output and exact JSON input forms for Wekan v11.06.
  Custom-field deletions report a requested, unconfirmed operation; missing
  definition lookups preserve HTTP 200 without invented server metadata. Card
  lookup rejects empty and dot-only values before HTTP instead of sending an
  incorrect endpoint. Definition and dropdown-item deletion prompts warn that
  deletion affects all boards associated with a shared field. Optional settings
  preserve explicit nulls in requests and accept nullable Wekan responses.
  Dot-only target IDs and settings whose dropdown item IDs become dot-only after
  Wekan trims whitespace are rejected before HTTP instead of losing selectors.
  Custom-field ID parsing preserves Wekan's U+0085 characters so distinct dropdown
  options remain distinct update and deletion targets.

## [0.5.0] - 2026-10-12

### Added

- Full `checklist` and `checklist-item` command families for creating, reading,
  updating items, and confirmed hard deletion, with typed human/JSON output
  and all corrected Wekan v11.06 input forms.

### Fixed

- Read UI-created checklists and checklist items through REST without rejecting
  their creator IDs, and expose optional `user_id` in detail output.
- Read checklists and items after REST card moves between boards while preserving
  the source `board_id` metadata returned by Wekan v11.06.

## [0.4.0] - 2026-10-11

### Added

- Secure raw stdin passwords for multiline and empty input, and SHA-256 password
  digest input for administrative user creation.
- Initial board owner administrator/active controls and a no-action user update.

### Fixed

- Reject unmapped workspace-node fields, including nested children, in typed
  user responses and preserve the known node fields in profile output.
- Preserve Wekan's accepted Unicode comment text during creation and reads,
  including comments containing only U+0085 (next line).
- Support partial WIP updates, nullable starred state, nullable/empty card
  creation text, Wekan date inputs and reversed user-card query ranges.
- Read back extended-year card dates, empty custom-field IDs and null labels;
  accept Wekan's custom-field value and coordinate coercions and text clearing.
- Preserve user profile settings and metadata in detail/action output, and send
  confirmed self-actions with Wekan v11.06's actual behavior.

### Security

- Escape terminal control characters in user profile map keys in human output.

## [0.3.0] - 2026-10-11

### Added

- Dedicated `card archive` and `card restore` commands, including recursive
  child-card changes and typed archive timestamps.
- Linked-card creation with `card create --linked-id`.
- Card updates for list/swimlane and cross-board movement, archive/restore,
  vote and poker settings, custom-field values, stickers, and locations.

### Fixed

- Missing linked source cards report `not_found`.
- Card detail output accepts null members in custom-field string arrays.

### Security

- Updated TLS handling to reject handshake messages sent at the wrong encryption
  level, fixing [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285)
  by upgrading rustls to 0.23.45.

## [0.2.1] - 2026-09-06

### Fixed

- Authentication status and user commands accept unread notifications without a
  read timestamp instead of failing with a protocol error.

## [0.2.0] - 2026-09-06

### Added

- Safe response field paths and failure categories in protocol errors.
- Command examples and grouped help for scoped resources and card updates.

### Changed

- Scoped list, swimlane, card, and comment commands use named parent selectors
  (`--board`, `--list`, and `--card`). The resource's own ID remains positional.

## [0.1.0] - 2026-08-30

### Added

- Profiles for connecting to different Wekan servers, with credentials stored
  securely by the operating system.
- Commands to register, log in, check authentication status, and log out,
  including two-factor authentication support.
- Commands to view and manage Wekan users.
- Commands to view and manage boards, lists, swimlanes, cards, and comments.
- Human-readable and JSON output.
- `wekan api request` for API operations without dedicated commands, including
  raw response output.
- Interactive confirmation prompts for destructive commands, with a `--yes`
  flag to skip the prompt.
- Strict response validation against the supported Wekan API contract.
