# Custom fields and dropdown items

The `custom-field` and `custom-field-item` families, together with two dedicated
card commands, cover all ten custom-field REST operations in Wekan v11.06.

```console
wekan custom-field list --board <BOARD_ID>
wekan custom-field get <CUSTOM_FIELD_ID> --board <BOARD_ID>
wekan custom-field create --board <BOARD_ID> --name <NAME> --type <TYPE> [--settings <JSON>] [--show-on-card <BOOL>] [--automatically-on-card <BOOL>] [--show-label-on-mini-card <BOOL>] [--show-sum-at-top-of-list <BOOL>]
wekan custom-field update <CUSTOM_FIELD_ID> --board <BOARD_ID> [--name <NAME>] [--type <TYPE>] [--settings <JSON>] [--show-on-card <BOOL>] [--automatically-on-card <BOOL>] [--always-on-card <BOOL>] [--show-label-on-mini-card <BOOL>] [--show-sum-at-top-of-list <BOOL>]
wekan custom-field delete <CUSTOM_FIELD_ID> --board <BOARD_ID> [--yes]
wekan custom-field-item create --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--item <TEXT>]... [--items <JSON>]
wekan custom-field-item update <ITEM_ID> --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--name <NAME>]
wekan custom-field-item delete <ITEM_ID> --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> [--yes]
wekan card set-custom-field <CARD_ID> --board <BOARD_ID> --list <LIST_ID> --custom-field <CUSTOM_FIELD_ID> [--value <TEXT>] [--value-json <JSON>]
wekan card find-by-custom-field --board <BOARD_ID> --custom-field <CUSTOM_FIELD_ID> --value <TEXT>
```

Every command accepts `--server <URL>`, `--profile <PROFILE>`,
`--output <human|json>`, `--allow-insecure-http`, and `--help`/`-h`.
Identifiers use Wekan's JavaScript whitespace trimming and must not become
empty. U+FEFF is trimmed; U+0085 remains part of an ID. Text is sent unchanged;
Wekan performs its own schema cleaning and validation. Collection order is preserved.
These commands reject target IDs `.` and `..` with `invalid_input` before
resolving credentials or sending HTTP because the URL library drops dot-only
path segments.

## Definition inputs

`--type` accepts `text`, `number`, `date`, `dropdown`, `checkbox`, `currency`,
and `stringtemplate`. Creation requires a name and type, and defaults settings
to `{}`. Display flags take explicit `true` or `false` values; omitted flags
use Wekan's false defaults. Creation has no `--always-on-card`: Wekan initializes
it false and ignores that property in creation requests.

`--settings` accepts a typed JSON object with these optional wire properties:

| Property | Value |
|---|---|
| `currencyCode` | String or null |
| `dropdownItems` | Array of objects, each requiring string `_id` and `name`, or null |
| `stringtemplateFormat` | String or null |
| `stringtemplateSeparator` | String or null |

Unknown keys, a null settings object, malformed JSON, and invalid types fail
argument parsing. Update replaces the entire supplied settings object; `{}`
clears its optional properties. Explicit null properties are sent unchanged and
retained by Wekan. There is no CLI merge or inferred type-specific
validation. Definition update accepts no fields and sends `{}`.

Supplied dropdown item IDs must not become `.` or `..` after Wekan's JavaScript
whitespace trimming, including U+FEFF. Creation and update reject such settings
before HTTP to avoid creating options the CLI cannot address. Wekan itself
accepts these IDs; existing definitions containing them remain readable, and
their settings can be replaced. IDs such as `...`, `%2E`, `./option`, and
`a/../b` remain supported and are encoded as individual path segments.

```console
wekan custom-field create --board board-id --name Budget --type currency --settings '{"currencyCode":"USD"}'
wekan custom-field update field-id --board board-id --show-on-card true --always-on-card false
```

## Dropdown inputs

Repeated `--item` flags send names in argument order and conflict with `--items`.
`--items` sends an exact JSON value. Wekan appends only string members from an
array; it ignores non-string members. Omitted items, a non-array value, or an
empty array produce a successful no-op. Empty item names can fail server
validation; the CLI does not trim or repair them. Update permits omitted name
and sends an empty object in that case.

Creation acknowledges the parent field ID. Use `custom-field get` to discover
generated six-character item IDs. Wekan has no separate item list/get endpoint.

```console
wekan custom-field-item create --board board-id --custom-field field-id --item Ready --item Done
wekan custom-field-item create --board board-id --custom-field field-id --items '["Ready",42,null,"Done"]'
wekan custom-field-item update item-id --board board-id --custom-field field-id --name Complete
```

## Card values and lookup

`card set-custom-field` calls the dedicated assignment endpoint. It only updates
entries already attached to an active card. An unattached field ID produces a
successful no-op. Attach entries using the existing `card update --custom-fields`
array replacement command when that is the intended operation.

`--value` sends a string and conflicts with `--value-json`, which sends exact
JSON. Omission sends no value property and Wekan defaults the existing entry to
an empty string; explicit null stays null. Wekan trims strings, converts objects
to an empty string, stringifies numeric/boolean array members, retains null
members, and removes object array members. Both the assignment acknowledgment
and stored values contain the cleaned values. Nested array members fail with
HTTP 500. The CLI does not coerce inputs or perform a readback.

Lookup passes an exact encoded string path value to Wekan. It excludes archived
cards, matches string values and members of string arrays, and does not convert
the query to a number or boolean. Numeric `2.5` therefore does not match a query
string `2.5`. Surrounding query whitespace is preserved, while stored values
have already undergone Wekan's trimming.

Lookup rejects empty values and the exact values `.` and `..` with
`invalid_input`, before resolving credentials or sending HTTP. The HTTP client's
URL library normalizes dot-only path segments, including percent-encoded forms.
Wekan itself accepts encoded dot-only values, but this CLI cannot query them.
Values such as `...`, ` . ` and literal `%2E` are preserved unchanged.

## Stable output and errors

JSON uses the existing `{"ok":true,"data":...}` envelope. Definition collections
contain `board_id` and `custom_fields` summaries with `custom_field_id`, `name`,
and `field_type`. Details expose every known property in snake case: IDs,
name/type, settings, all display controls, and timestamps. Settings expose
`currency_code`, `dropdown_items` (with `item_id` and `name`),
`stringtemplate_format`, and `stringtemplate_separator`. Absent optional
properties and explicit null settings values both appear as null.

Creation returns the board and acknowledged field ID. Updates include scope IDs
and `submitted_fields`, ordered `name`, `type`, `settings`, `show_on_card`,
`automatically_on_card`, `always_on_card`, `show_label_on_mini_card`, then
`show_sum_at_top_of_list`; item update reports only `name`. Empty updates report
an empty list. Card assignment includes all scope IDs and the complete cleaned
`custom_fields` array. Lookup includes `board_id`, `custom_field_id`, `value`,
and full card details in `cards`.

Response objects reject additional fields, including settings, dropdown items,
card values, full lookup cards, and mutation acknowledgments. Unmapped fields,
invalid types, missing required values, invalid dates, and mismatched
acknowledgment IDs produce explicit protocol errors. Successful mutation
responses that cannot be mapped report `outcome_unknown: true`.

A missing or wrongly scoped definition returns an empty HTTP 200 body, mapped
to `not_found`, with `http_status: 200` and no invented Wekan status or error
fields. Missing, wrongly listed, and archived cards on assignment return
HTTP 500, preserved as `server_error`. Definition updates can save an earlier
field before a later validation failure; these errors report an unknown outcome.

## Permissions and deletion

Normal active members can read and mutate. Read-only members can read but cannot
mutate. Comment-only, worker, and no-comments members fail these access gates;
site administrators bypass membership checks. The CLI preserves server gates.

Both deletes require confirmation or command-local `--yes`; JSON and
non-terminal invocations require `--yes`. Cancellation succeeds with operation
`custom_field_delete` or `custom_field_item_delete`. Success contains scope IDs,
`deletion_requested: true`, `mutation_confirmed: false`, and
`delete_mode: "hard"`. These acknowledge the requested operation without claiming
that a document or item was removed. No readback is performed. Dropdown deletion
acknowledges the parent field ID. Updates and item creation likewise can
acknowledge missing targets without changing anything.

Deleting a shared definition through one board removes the definition from all
associated boards and removes its values from their cards. Deleting a dropdown
item changes the shared definition's options for every associated board, even
when the command selects only one board. Both confirmation prompts disclose
this scope and identify the requested board. REST definition updates use direct
collection writes: `alwaysOnCard: true` does not populate existing cards through
the UI's normal hooks. The CLI performs no repair, preflight, automatic retry,
rollback, or hook emulation.
