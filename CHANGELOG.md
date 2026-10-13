# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
