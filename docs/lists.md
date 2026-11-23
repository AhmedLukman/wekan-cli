# Lists

The singular `wekan list` family covers core list operations plus native
copying, moving, and repositioning. See [copy and move](copy-and-move.md) for
transfer syntax, returned IDs, and native transfer behavior.

## Commands

```console
wekan list list --board <BOARD_ID>
wekan list get <LIST_ID> --board <BOARD_ID>
wekan list create --board <BOARD_ID> --title <TITLE> [--swimlane-id <ID>]
wekan list update <LIST_ID> --board <BOARD_ID> [--title <TITLE>] [--color <COLOR>] [--starred <true|false> | --clear-starred] [--wip-limit <NUMBER>] [--wip-enabled <true|false>] [--wip-soft <true|false>]
wekan list delete <LIST_ID> --board <BOARD_ID> [--yes]
```

Titles and identifiers must remain non-empty after trimming where applicable.
Wekan v13.28 truncates update titles after the first 1000 JavaScript UTF-16 code
units; the CLI submits longer titles unchanged so that pinned behavior remains
visible. Update colors use Wekan v13.28's list palette; custom colors have
exactly six hexadecimal digits after `#`. Wekan can return `color: ""` for a
persisted unset color; `list get` preserves that response form, while the
update flag accepts only named palette colors or `#rrggbb`. Any combination of WIP options is accepted. Wekan replaces the whole
WIP object, defaulting omitted `value` to 1 and omitted `enabled`/`soft` to false;
it does not merge with the previous settings. `--clear-starred` sends null and
conflicts with `--starred`.

Use `--output json` for stable machine-readable envelopes. A collection result
has the shape:

```json
{
  "ok": true,
  "data": {
    "board_id": "board-id",
    "lists": [
      {
        "list_id": "list-id",
        "title": "Todo",
        "modified_at": "2030-01-02T03:04:05Z",
        "cards_modified_at": null
      }
    ]
  }
}
```

`list get` maps the complete supported list document to snake-case output. In
particular, `_id` becomes `list_id`, `_updatedAt` becomes `position_updated_at`,
and `type` becomes `list_type`. Optional scalars use their value or JSON null;
`list_type` remains an open string. Unknown top-level or WIP fields, invalid
timestamps, invalid nonempty colors, widths outside 100â€“1000, and empty required
identifiers cause `protocol_error`. `watchers` is `null` when absent and preserves an array when present,
including `[]` after the last watcher is removed.

Creation returns `{"board_id":"...","list_id":"..."}`.

An update reports its accepted fields in the fixed order `title`, `color`,
`starred`, `wip_limit`, regardless of option order. For example:

```json
{
  "ok": true,
  "data": {
    "board_id": "board-id",
    "list_id": "list-id",
    "updated_fields": ["title", "starred", "wip_limit"]
  }
}
```

## Soft deletion

Wekan soft-deletes the list and its live cards in one batch. The command
requires interactive confirmation or `--yes`. Success includes `board_id`, `list_id`,
`deleted: true`, `deletion_requested: true`, and `delete_mode: "soft"`. A second
deletion or a list ID absent from the selected board returns HTTP 404 and
`not_found`. Declining the confirmation returns the `list_delete` cancellation.

A soft-deleted list remains readable through `list get`, including its deletion
metadata. The active collection filters archived lists and can include
soft-deleted lists. A lookup for a nonexistent list returns HTTP 404.

Copy/move command syntax, position inputs, typed acknowledgments, and pinned
server behavior are documented in [copy and move](copy-and-move.md).

## Archiving and restoring

`wekan list archive LIST_ID --board BOARD_ID` archives a list through its native
operation. `wekan list restore LIST_ID --board BOARD_ID` restores it. Archived
lists are excluded from active reads; restoring makes the list readable again.
Both operations report the returned list ID and archive state.
