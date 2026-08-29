# Lists

The singular `wekan list` command family covers only the five core list
operations. It does not copy, move, order, archive, restore, or operate on list
cards and other subresources.

## Commands

```console
wekan list list <BOARD_ID>
wekan list get <BOARD_ID> <LIST_ID>
wekan list create <BOARD_ID> --title <TITLE> [--swimlane-id <ID>]
wekan list update <BOARD_ID> <LIST_ID> [--title <TITLE>] [--color <COLOR>] [--starred <true|false>] [--wip-limit <NUMBER> --wip-enabled <true|false> --wip-soft <true|false>]
wekan list delete <BOARD_ID> <LIST_ID> [--yes]
```

Titles and identifiers must remain non-empty after trimming where applicable.
Wekan v11.06 truncates update titles after the first 1000 JavaScript UTF-16 code
units; the CLI submits longer titles unchanged so that pinned behavior remains
visible. Update colors use Wekan v11.06's list palette; custom colors have
exactly six hexadecimal digits after `#`. Wekan can return `color: ""` for a
persisted unset color; `list get` preserves that response form, while the
update flag accepts only named palette colors or `#rrggbb`. The three WIP
options are one field and must always be supplied together.

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
particular, Wekan's `_updatedAt` field becomes `position_updated_at` and its
`type` field becomes `list_type`. Responses are strict: an unmodeled field at
the list or nested WIP level is a protocol error rather than silently discarded
data.

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

Wekan v11.06 does not physically remove a list. It stamps the list and its live
cards with deletion metadata in one batch. Because cards are affected,
`list delete` requires interactive confirmation or command-local `--yes`.

The operation is idempotent: deleting an already deleted or absent list returns
the requested ID just like a newly deleted list. Therefore `deleted: true` and
`delete_mode: "soft"` mean Wekan accepted and validated the request, not that a
previously live list was found.

Wekan v11.06 has two related read quirks. Its board-list collection filters
archived lists but not soft-deleted lists and omits deletion metadata from each
summary, so a deleted list may remain in `list list` output. Its missing
single-list route returns an empty HTTP 200 response; the CLI normalizes that to
the stable `not_found` error.
