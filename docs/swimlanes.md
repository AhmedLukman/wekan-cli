# Swimlanes

The singular `wekan swimlane` family covers core swimlane operations plus
native copying, moving, and repositioning. See [copy and move](copy-and-move.md)
for transfer syntax and acknowledgments. Use `card list --board <BOARD_ID>
--swimlane <SWIMLANE_ID>` to list cards in a swimlane.

## Commands

```console
wekan swimlane list --board <BOARD_ID>
wekan swimlane get <SWIMLANE_ID> --board <BOARD_ID>
wekan swimlane create --board <BOARD_ID> --title <TITLE> [--sort <NUMBER>]
wekan swimlane update <SWIMLANE_ID> --board <BOARD_ID> --title <TITLE>
wekan swimlane delete <SWIMLANE_ID> --board <BOARD_ID> [--yes]
```

Titles and identifiers are trimmed and must not be empty. `--sort` accepts any
finite JSON number, including negative and fractional values. When omitted,
Wekan appends the new swimlane after the existing swimlanes.

Use `--output json` for stable machine-readable envelopes. A collection result
has the shape:

```json
{
  "ok": true,
  "data": {
    "board_id": "board-id",
    "swimlanes": [
      {
        "swimlane_id": "swimlane-id",
        "title": "Delivery"
      }
    ]
  }
}
```

`swimlane get` maps the complete supported Wekan v11.06 document to
snake-case output. Wekan's `_id` becomes `swimlane_id` and `type` becomes
`swimlane_type`. Optional archive/update dates, sort, color and height are
present as their value or JSON `null`. Dates, colors, Wekan's `-1` or
50-through-2000 height constraint, and required nonempty identifiers are
validated. Invalid known values or unmodeled fields cause `protocol_error`.

Creation returns `{"board_id":"...","swimlane_id":"..."}`.
An update reports the one supported field explicitly:

```json
{
  "ok": true,
  "data": {
    "board_id": "board-id",
    "swimlane_id": "swimlane-id",
    "updated_fields": ["title"]
  }
}
```

## Hard deletion

Wekan v11.06 physically removes the swimlane. When fewer than two matching live
lists exist, its removal hook physically removes those lists and their cards;
otherwise it removes cards assigned to the swimlane and leaves the lists.
Because those effects are destructive, `swimlane delete` requires interactive
confirmation or command-local `--yes`.

The endpoint is idempotent: deleting an absent swimlane still returns the
requested ID. Therefore `deletion_requested: true` and `delete_mode: "hard"` mean Wekan
accepted and validated the request, not that a live swimlane necessarily
existed immediately before the call. Delete data includes `board_id` and
`swimlane_id`. Declining deletion returns the shared cancellation result with
operation `swimlane_delete`. A subsequent `swimlane get` normalizes
Wekan's empty HTTP 200 response to the stable `not_found` error.

Copy/move command syntax, position inputs, typed acknowledgments, and pinned
server behavior are documented in [copy and move](copy-and-move.md).
