# Card dependencies

The root `wekan dependency` family manages the five dependency REST operations
in Wekan v12.57. A dependency is stored on a source card and references a target
card. The commands require board IDs, with no list selector.

```console
wekan dependency list --board <BOARD_ID> [--card <CARD_ID>]
wekan dependency create <TARGET_CARD_ID> --board <BOARD_ID> --card <CARD_ID> [--type <TYPE> | --null-type] [--color <COLOR> | --null-color] [--icon <ICON> | --null-icon]
wekan dependency update <TARGET_CARD_ID> --board <BOARD_ID> --card <CARD_ID> [--type <TYPE> | --null-type] [--color <COLOR> | --null-color] [--icon <ICON> | --null-icon]
wekan dependency delete <TARGET_CARD_ID> --board <BOARD_ID> --card <CARD_ID> [--yes]
```

All commands accept `--server <URL>`, `--profile <PROFILE>`,
`--output <human|json>`, `--allow-insecure-http`, and `--help`/`-h`.
Only deletion accepts `--yes`. IDs are trimmed and must be nonempty. Dot-only
selectors are rejected because the HTTP client's URL library cannot preserve
those path segments. Metadata strings are not trimmed.

## Listing and direction

`list --board` calls the board endpoint, which includes lines from active source
cards. `list --board --card` calls the card endpoint, which returns that card's
outgoing dependencies even when the source card is archived. Both retain the
server's order. Read access is required; mutations require board write access.

The relation types are `related-to`, `blocks`, `is-blocked-by`, `fixes`, and
`is-fixed-by`. They describe the relation from the stored source card's
perspective. Creating a dependency does not add a reciprocal edge to the target.
The CLI does not infer incoming dependencies or reverse relation types.

## Metadata and mutations

`create` uses POST with the canonical `cardId` target property. It creates a new
edge or merges metadata into an existing edge without duplicating it. Omitted,
empty, or null metadata retains the existing value, or uses Wekan's defaults:
`related-to`, `#eb144c`, and `link`.

`update` uses PUT and changes only submitted properties. `--type` also accepts
an empty string; `--color` and `--icon` accept any string, including an empty
one. Use `--type=`, `--color=`, or `--icon=` to express an empty value portably.
`--null-type`, `--null-color`, and `--null-icon` send JSON null. Each null flag
conflicts with its corresponding value flag.

Wekan's schema cleaning removes empty strings from PUT metadata; explicit nulls
persist. The dependency listing endpoints normalize missing/null metadata back
to defaults. `wekan card get` returns the raw stored metadata,
so its values can differ from `dependency list` (absent/null properties both
render as null in card output). Raw card dependency types are
optional nullable strings; normalized dependency responses require a named type.

An update without metadata sends `{}`. Wekan v12.57 returns HTTP 500 with the
symbolic `bad-request` error. Missing source cards also return HTTP 500 with
`not-found`; self, missing, or foreign-board POST targets return HTTP 500 with
`bad-request`. The CLI preserves these statuses and server details rather than
substituting local existence checks or retries.

PUT and DELETE can acknowledge a nonexistent edge without changing anything.
They return only the source card ID, with no matched count. Deleting a target
card leaves its incoming dependencies; board listings then return null target
title/card number. A dangling edge can still be removed with dependency delete.

Deletion asks for confirmation, or requires `--yes` in automation. The stored
credential is checked again after confirmation; removal, replacement, or expiry
prevents the HTTP request. Declining produces the normal cancellation result.

## Stable output

JSON uses the common `{"ok":true,"data":...}` envelope. Each successful command
returns only mapped typed fields. Unknown response properties, invalid types,
missing required fields, or mismatched acknowledgment IDs produce
`protocol_error`. Errors retain the known status and response diagnostics.

- Card listing: `board_id`, `card_id`, and `dependencies`, with each entry's
  `target_card_id`, `dependency_type`, `color`, and `icon`.
- Board listing: `board_id` and `dependencies`, with each entry's
  `source_card_id`, `source_title`, `source_card_number`, `target_card_id`,
  `target_title`, `target_card_number`, `dependency_type`, `color`, and `icon`.
  Missing/null descriptive fields render as null; numeric values retain precision.
- Create: `board_id`, `card_id`, `target_card_id`, `dependency_type`, `color`, and
  `icon`, using the server-returned metadata. It does not claim the edge was new.
- Update: `board_id`, `card_id`, `target_card_id`, `submitted_fields` (in type,
  color, icon order).
- Delete: `board_id`, `card_id`, and `target_card_id`.

Human output reports the completed action and escapes terminal controls.

The pinned contract and normalization behavior are implemented in
[`server/models/dependencies.js`](https://github.com/wekan/wekan/blob/bee4e71d86d0ef2669f9a389d77aba03fdb8103f/server/models/dependencies.js)
and
[`models/metadata/dependencies.js`](https://github.com/wekan/wekan/blob/bee4e71d86d0ef2669f9a389d77aba03fdb8103f/models/metadata/dependencies.js).

Update and deletion report request acceptance because a missing edge is a successful no-op. Creation retains direct create-or-update wording.

Declining dependency deletion returns the shared cancellation result with
operation `dependency_delete`.
