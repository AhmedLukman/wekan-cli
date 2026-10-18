# Board automation rules

`wekan rule` manages the five native Wekan v11.06 rule endpoints. A rule links a
trigger to an action. Commands use REST and accept `--output human` or `json`,
plus the usual server/profile options.

## Commands and JSON definitions

```console
wekan rule list --board BOARD_ID
wekan rule get RULE_ID --board BOARD_ID
wekan rule create --board BOARD_ID --rule-file rule.json
wekan rule update RULE_ID --board BOARD_ID --rule '{"title":"New title"}'
wekan rule delete RULE_ID --board BOARD_ID --yes
```

Create and update require exactly one of `--rule <JSON>`, `--rule-file <PATH>`,
or `--rule-stdin`. Inputs are UTF-8 JSON objects bounded to 50 MiB. Files must
be regular files. Invalid JSON, unsupported properties, or invalid field types
fail before credentials or HTTP with `invalid_input` (exit 2). File read failures
use `file_io_error` (exit 4). No YAML conversion or field-flag input is provided.

Successful get/list responses are bounded to 64 MiB, allowing large definitions
plus server defaults and response metadata. A board's combined rule listing can
still exceed this limit and fail with `protocol_error` (exit 4).

Definitions use Wekan camelCase names. Creation requires `trigger` and `action`
objects with nonempty string `activityType` and `actionType` values. `title` is
optional; Wekan uses `API rule` for an omitted or empty creation title.
Updates accept any subset of `title`, `trigger`, and `action`, including `{}`.
Top-level title must be a string when present; trigger/action must be objects.

For example, save this as `rule.json` to add a member when a card moves to Doing:

```json
{
  "title": "Add member on move to Doing",
  "trigger": {"activityType": "moveCard", "listName": "Doing"},
  "action": {"actionType": "addMember", "username": "alice"}
}
```

A companion rule removes that member when the card moves away:

```json
{
  "title": "Remove member on move from Doing",
  "trigger": {"activityType": "moveCard", "oldListName": "Doing"},
  "action": {"actionType": "removeMember", "username": "alice"}
}
```

## Supported payload fields

The typed commands map all built-in v11.06 payload fields. Discriminators are
strings, so the CLI preserves unknown kind names instead of promising they run.
Other unmapped fields or object/array values in scalar fields are rejected;
use `wekan api request` for arbitrary custom payloads.

| Payload | Supported camelCase properties |
| --- | --- |
| Trigger matching | `description`, `activityType`, `userId`, `username`, `listName`, `oldListName`, `swimlaneName`, `cardTitle`, `checklistName`, `checklistItemName`, `labelId`, `attachmentName`, `desc` |
| Trigger schedules | `scheduleKind`, `scheduleType`, `atTime`, `weekday`, `dayOfMonth`, `onDate`, `dueCondition`, `days`, `lastRunKey`, `lastRunAt` |
| Trigger buttons | `buttonType`, `buttonLabel` |
| Action | `actionType`, `listName`, `swimlaneName`, `cardName`, `fromListName`, `checklistName`, `checkItemName`, `checklistItems`, `dateField`, `unit`, `emailTo`, `emailSubject`, `emailMsg`, `selectedColor`, `labelId`, `username`, `sortField`, `desc`, `days` |

All payload fields are sparse and nullable. `weekday`, `dayOfMonth`, and `days`
accept JSON numbers or strings; other listed properties accept strings.
The CLI preserves numeric text without applying Wekan's coercions.
`lastRunAt` in requests and responses must be a valid Wekan date-time string or
null. Invalid request timestamps fail with `invalid_input` before credentials or
HTTP; valid timestamp text is preserved, including offsets and extended years.
Requests also accept `_id`, `boardId`, `createdAt`, `modifiedAt`, and `updatedAt`
as nullable strings on trigger/action objects. Wekan strips these properties,
attaches the selected board ID, and generates its own IDs and timestamps.
Responses omit those stripped fields; scheduled runtime `lastRunKey` and
`lastRunAt` remain visible.

Activity kinds are `createCard`, `moveCard`, `archivedCard`, `restoredCard`,
`joinMember`, `unjoinMember`, `addChecklist`, `removeChecklist`,
`completeChecklist`, `uncompleteChecklist`, `addedChecklistItem`,
`removedChecklistItem`, `checkedItem`, `uncheckedItem`, `addAttachment`,
`deleteAttachment`, `addedLabel`, and `removedLabel`. `scheduledTrigger` and
`button` are evaluated separately from activities.

Action kinds are `moveCardToTop`, `moveCardToBottom`, `sendEmail`, `setDate`,
`updateDate`, `removeDate`, `setDateRelative`, `archive`, `unarchive`, `setColor`,
`addLabel`, `removeLabel`, `addMember`, `removeMember`, `checkAll`, `uncheckAll`,
`checkItem`, `uncheckItem`, `addChecklist`, `removeChecklist`,
`addChecklistWithItems`, `addSwimlane`, `createCard`, `linkCard`,
`markCardComplete`, `markCardIncomplete`, `sortList`, and `moveAllCardsInList`.

## Native behavior and limitations

Wekan fills omitted, null, or empty matching fields with `*` when an activity
kind is supplied. On update, fields are shallow-merged with `$set`; omitted
stored fields remain. Including `activityType` resets omitted matching fields
for that kind to wildcards. A partial update without `activityType` does not
perform that normalization. Changing kinds can retain fields from the previous
kind. Explicit nulls and empty strings are sent unchanged for Wekan to handle.

Reads require board access. Writes use Wekan's board-write permission check,
which permits normal writable members as well as administrators. Read-only
members can read but cannot mutate. Missing boards retain HTTP 404; missing or
wrongly scoped rules retain HTTP 500 `not-found`, surfaced as `server_error`
(exit 5), rather than being translated into HTTP 404.

Each command sends one native request. Creation, update, and deletion perform
sequential server writes. Failures may leave earlier effects, and mutation
errors preserve their actual status and diagnostic details. There is no
automatic retry or rollback.

REST overwrites an action's `boardId` with the selected rule board and strips it
from reads. Cross-board destination selection in the UI is not reproduced by
these REST endpoints. Button fields persist on the trigger, but REST creation
does not copy button metadata onto the rule document as the UI creation method
does. Such definitions do not supply the metadata needed for UI button rendering.
Manual button execution is a DDP method and has no equivalent in this family.

Scheduled definitions use Wekan's server clock and minute-scanner format.
However, pinned v11.06 registers the scanner without starting SyncedCron;
scheduled rules persist but do not execute on the default stack (W11-053).
The CLI preserves this defect without starting or emulating the scheduler.
For example, this definition persists with the intended daily schedule:

```json
{
  "trigger": {"activityType":"scheduledTrigger","scheduleKind":"calendar","scheduleType":"daily","atTime":"09:00","listName":"Todo"},
  "action": {"actionType":"setColor","selectedColor":"purple"}
}
```

Schedule kinds include `calendar`, `due`, and `aging`. Calendar types include
`once`, `daily`, `weekday`, `weekly`, and `monthly`; due conditions include
`set`, `soon`, and `overdue`. The CLI does not evaluate schedules or validate
server-side matching/action semantics.

## Stable output and deletion

List data contains `board_id` and `rules`. Each rule detail contains `board_id`,
`rule_id`, `title`, `trigger`, and `action`. Nested keys use snake_case, preserve
explicit nulls, and omit fields absent from Wekan. Missing referenced documents
are represented as null. Creation returns `board_id`, `rule_id`, `trigger_id`,
and `action_id`. Update returns `board_id`, `rule_id`, `submitted_fields`
(in title/trigger/action order).
Deletion returns `board_id`, `rule_id`, and `deleted: true` after the native
handler completes removal of the rule and its referenced documents.

Deletion prompts on a terminal or accepts `--yes`; a decline returns the usual
cancellation result with operation `rule_delete`. Credentials are rechecked
after confirmation. Create/update run directly. Rule reads reject additional
fields at every object boundary with `protocol_error` (exit 4); a custom field
on one rule also causes the complete board rule listing to fail.
