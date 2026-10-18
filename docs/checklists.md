# Checklists and checklist items

The `wekan checklist` family and its `item` subcommands cover all eight checklist
operations exposed by Wekan v11.06:

```console
wekan checklist list --board <BOARD_ID> --card <CARD_ID>
wekan checklist get <CHECKLIST_ID> --board <BOARD_ID> --card <CARD_ID>
wekan checklist create --board <BOARD_ID> --card <CARD_ID> [--title <TEXT>] [--item <TEXT>]... [--items <JSON>]
wekan checklist delete <CHECKLIST_ID> --board <BOARD_ID> --card <CARD_ID> [--yes]
wekan checklist item get <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID>
wekan checklist item create --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> --title <TEXT> [--sort <NUMBER>] [--sort-json <JSON>]
wekan checklist item update <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> [--title <TEXT>] [--is-finished <BOOL>] [--is-finished-json <JSON>]
wekan checklist item delete <ITEM_ID> --board <BOARD_ID> --card <CARD_ID> --checklist <CHECKLIST_ID> [--yes]
```

Every command also accepts `--server <URL>`, `--profile <PROFILE>`,
`--output <human|json>`, `--allow-insecure-http`, and `--help`/`-h`.
Identifiers are trimmed and must not be blank. Text is sent unchanged, including
empty strings; Wekan performs its own cleaning and validation. Successful reads
preserve returned titles and collection order.

Wekan has no REST endpoint for updating a checklist or listing items separately.
Use `checklist get` to read its item summaries, then `checklist item get` for an
item's full metadata.

## Creation and exact input forms

Checklist creation defaults to the title `Checklist` when no title is supplied
or the supplied title is empty.
Repeated `--item` flags send an array of initial item titles in argument order.
They conflict with `--items`, which accepts a JSON string, string array, or null.
Omitted items, null, an empty string, and an empty array create no items; a
nonempty singleton string creates one item. Wekan assigns the checklist sort 0
and initial item sorts 0, 1, and so on.

```console
wekan checklist create --board board-id --card card-id --title Release --item Verify --item Ship
wekan checklist create --board board-id --card card-id --items '["Verify","Ship"]'
wekan checklist create --board board-id --card card-id --items '"Verify"'
wekan checklist create --board board-id --card card-id --items null
```

Item creation requires a title. Omitted sort appends after the current maximum
sort, or uses 0 for an empty checklist. `--sort` accepts finite numbers, including
negative and fractional values. It conflicts with `--sort-json`, which sends an
exact JSON number, string, or null. Wekan number-coerces that value: numeric
strings set a position, null and an empty string become 0, and nonfinite input
such as `"Infinity"` or `"NaN"` appends. The CLI does not coerce these JSON values.
New items start unchecked.

```console
wekan checklist item create --board board-id --card card-id --checklist checklist-id --title Verify
wekan checklist item create --board board-id --card card-id --checklist checklist-id --title Verify --sort -1.5
wekan checklist item create --board board-id --card card-id --checklist checklist-id --title Verify --sort-json '"4.5"'
```

Item update accepts any subset of title and completion, including no fields.
An empty update sends `{}` and acknowledges the existing item. Boolean
`--is-finished` conflicts with `--is-finished-json`, which accepts a JSON boolean
or string. Wekan treats case-insensitive `"true"` as true and every other string
as false; it does not trim the completion string. Item sort cannot be updated
through this endpoint.

## Stable output

JSON output uses the usual `{"ok":true,"data":...}` envelope. Checklist list data
contains `board_id`, `card_id`, and `checklists`, with `checklist_id` and `title`
on each summary. Checklist detail maps every known field:

```json
{"checklist_id":"checklist-id","card_id":"card-id","board_id":"board-id","user_id":"creator-id","title":"Release","finished_at":null,"created_at":"2030-01-01T00:00:00Z","modified_at":"2030-01-01T00:00:00Z","sort":0,"hide_checked_checklist_items":null,"hide_all_checklist_items":null,"show_checklist_at_minicard":null,"items":[{"item_id":"item-id","title":"Verify","is_finished":false}]}
```

Item detail contains `item_id`, `title`, `sort`, `is_finished`, `checklist_id`,
`card_id`, `board_id`, `user_id`, `created_at`, and `modified_at`. Both detail
commands include the optional creator `user_id`: Wekan UI/DDP creation populates
it, while REST creation omits it. The CLI reads both kinds through REST; absent
creator IDs appear as null in JSON and are omitted in human output. Optional
absent or null metadata is represented as null. Required fields remain required. All response
objects reject additional fields, including nested summaries and mutation
acknowledgements. Returned checklist, card, and item IDs must match the requested
scope. The optional `board_id` preserves Wekan's denormalized metadata, which can
retain the source board after a REST card move. Timestamps must be valid Wekan
date-times.

Creation data includes the requested board/card scope and returned checklist ID,
or the checklist scope and returned item ID. Update data includes all scope IDs,
`item_id`, and `submitted_fields`, ordered `title` then `is_finished`. Empty
updates return `submitted_fields: []`. Human output reports the completed
update. Deletion data includes the same scope IDs,
`deleted: true`, and `delete_mode: "hard"`.

## Verified Wekan v11.06 behavior

- A REST card move to another board leaves existing checklist and item `boardId`
  metadata on the source board. Read these resources using the destination board
  selector; detail output preserves the returned source `board_id`. Wekan scopes
  and authorizes these requests through the card's current board.
- A missing or wrongly scoped card returns 404. A missing or wrongly scoped
  checklist on an existing card returns an empty HTTP 500 on checklist lookup;
  the CLI preserves it as `server_error`. Missing or wrongly scoped items return
  404 and become `not_found`.
- Normal members can use all operations. Read-only members can read, mutate
  items, and delete checklists; creating a checklist additionally checks comment
  capability and rejects read-only members. Comment-only, worker, and no-comments
  members fail the general access check. Site administrators bypass membership
  checks. These are the pinned server's permission rules.
- Checklist creation inserts the checklist before inserting initial items. A
  later invalid item can fail with HTTP 500 after the checklist and earlier
  items were saved. Item update writes completion before title and can likewise
  fail title validation after saving completion. The CLI reports the error
  without rollback.
- Item updates and deletion use Wekan's direct collection operations, bypassing
  activity/completion hooks. The CLI does not emulate those hooks or infer a
  checklist completion timestamp from checked items.
- Checklist deletion leaves its item documents behind. An existing orphan item
  remains readable and deletable by its original checklist ID while its card
  exists. The CLI sends only the requested deletion and performs no cleanup.

Both deletion commands require interactive confirmation or command-local
`--yes`. JSON and non-terminal invocations require `--yes`. Declining a prompt
returns exit 0 with a cancellation operation of `checklist_delete` or
`checklist_item_delete`. Malformed or mismatched successful mutation responses
produce `protocol_error`; mutations are never
retried automatically.
