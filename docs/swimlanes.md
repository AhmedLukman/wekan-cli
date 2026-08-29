# Swimlanes

The singular `wekan swimlane` command family covers only the five core
swimlane operations. It does not copy, move, order, archive, restore, operate
on cards, or expose other swimlane subresources.

## Commands

```console
wekan swimlane list <BOARD_ID>
wekan swimlane get <BOARD_ID> <SWIMLANE_ID>
wekan swimlane create <BOARD_ID> --title <TITLE> [--sort <NUMBER>]
wekan swimlane update <BOARD_ID> <SWIMLANE_ID> --title <TITLE>
wekan swimlane delete <BOARD_ID> <SWIMLANE_ID> [--yes]
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
preserved. Dates, colors and Wekan's `-1` or 50-through-2000 height constraint
are validated. Any unmodeled response field is a protocol error rather than
being silently discarded.

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
requested ID. Therefore `deleted: true` and `delete_mode: "hard"` mean Wekan
accepted and validated the request, not that a live swimlane necessarily
existed immediately before the call. A subsequent `swimlane get` normalizes
Wekan's empty HTTP 200 response to the stable `not_found` error.
