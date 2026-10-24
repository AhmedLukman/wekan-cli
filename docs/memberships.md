# Board and card memberships

The authenticated membership commands operate on existing Wekan user IDs.
They use the selected profile and stored credential, accept human or JSON
output, and do not resolve names or send invitations.

```text
wekan board member list --board <BOARD_ID>
wekan board member add <USER_ID> --board <BOARD_ID> [--role <ROLE>] [--is-admin <BOOL>] [--is-no-comments <BOOL>] [--is-comment-only <BOOL>] [--is-worker <BOOL>] [--is-normal-assigned-only <BOOL>] [--is-comment-assigned-only <BOOL>] [--is-read-only <BOOL>] [--is-read-assigned-only <BOOL>]
wekan board member remove <USER_ID> --board <BOARD_ID>
wekan board member update <USER_ID> --board <BOARD_ID> [--role <ROLE>] [--is-admin <BOOL>] [--is-no-comments <BOOL>] [--is-comment-only <BOOL>] [--is-worker <BOOL>] [--is-normal-assigned-only <BOOL>] [--is-comment-assigned-only <BOOL>] [--is-read-only <BOOL>] [--is-read-assigned-only <BOOL>]

wekan card member list --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
wekan card member add <USER_ID> --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
wekan card member remove <USER_ID> --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
wekan card assignee list --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
wekan card assignee add <USER_ID> --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
wekan card assignee remove <USER_ID> --board <BOARD_ID> --list <LIST_ID> --card <CARD_ID>
```

Every command additionally accepts `--server <URL>`, `--profile <PROFILE>`,
`--output human|json`, `--allow-insecure-http`, and `--help`/`-h`. Update requires
at least one permission input. Boolean flags take explicit `true` or `false`.
Removals execute directly and do not accept `--yes`. IDs must be nonblank and
cannot be `.` or `..`, which the HTTP URL library cannot preserve as segments.

## Board permissions and membership state

`--role` accepts these case-insensitive names, with surrounding JavaScript
whitespace trimmed:

| Names | Enabled flag |
| --- | --- |
| `normal`, `member` | None |
| `admin` | `is_admin` |
| `comment`, `commentonly` | `is_comment_only` |
| `nocomments` | `is_no_comments` |
| `worker` | `is_worker` |
| `normalassignedonly` | `is_normal_assigned_only` |
| `commentassignedonly` | `is_comment_assigned_only` |
| `readonly` | `is_read_only` |
| `readassignedonly` | `is_read_assigned_only` |

A named role sets its flag and clears the others. It takes precedence over all
individual flags supplied in the same request. Without a role, add interprets
omitted flags as false; adding without permissions creates a normal member.
Update sends only the supplied inputs to Wekan's direct permission setter.
It does not read and merge existing permissions: omitted `is_normal_assigned_only`,
`is_comment_assigned_only`, `is_read_only`, and `is_read_assigned_only` reset to
false. Use a named role or supply all eight flags when setting a complete
permission combination. Updating yourself preserves your current admin flag.

Board add reactivates an inactive member with the submitted permissions. Adding
an already active member leaves that member's permissions unchanged. Remove
deactivates the member and clears its admin flag; it preserves the board's sole
active administrator. It does not delete user accounts or erase inactive
membership records. Disabled users cannot be added.

Board list includes all membership records in server order, including inactive
ones. It uses the complete strict board detail response and exposes the same
snake-case member fields as `board get`, with absent optional flags as null.
Board mutations require board-administrator or site-administrator permission.

## Card assignments

The focused lists reuse the complete strict card detail endpoint and return
only the selected member or assignee array, in server order. Mutations use
Wekan's individual merge endpoints and return both stored arrays from the
requested card. Duplicate adds and repeated removal of a known user are
idempotent. Other assignments are preserved; there is no read/replace/write
workflow.

Adding requires an active board membership for the target user. Off-board,
inactive, and nonexistent IDs return HTTP 400. Removing a known former board
member remains permitted. Wekan v12.57 removal of a nonexistent user fails with
HTTP 500 HTML through a model hook; the CLI preserves that server error. Wrong
list/card selectors and archived cards return HTTP 404 for these mutations.
Card assignment mutations require Wekan board write access.

For a linked card, Wekan v12.57 changes the source card's assignments but
returns the link's ID and its own stored arrays copied when the link was created.
They can remain empty or retain older assignments. Focused lists also return the
link's stored arrays. These snapshots do not confirm the source card's saved
assignments. This is a pinned server defect; read the source
card explicitly to inspect its assignments.

Existing `card create --member/--assignee` and `card update
--member/--assignee/--clear-members/--clear-assignees` retain their complete-array
semantics. Their ID-only acknowledgments do not prove every submitted ID was
stored. Individual endpoints return snapshots of the requested card's arrays,
subject to the linked-card limitation above.

## Stable results and errors

Successful commands use `{ "ok": true, "data": ... }`. Representative data:

```json
{"board_id":"b","members":[{"user_id":"u","is_admin":false,"is_active":false,"is_no_comments":false,"is_comment_only":false,"is_worker":false,"is_normal_assigned_only":false,"is_comment_assigned_only":false,"is_read_only":true,"is_read_assigned_only":false,"granted_by":null,"custom_role_id":null}]}
{"board_id":"b","user_id":"u","action":"add","boards":[{"board_id":"b","title":"Planning"}]}
{"board_id":"b","user_id":"u","affected_documents":1,"submitted_fields":["role"]}
{"board_id":"b","list_id":"l","card_id":"c","members":["u"]}
{"board_id":"b","list_id":"l","card_id":"c","assignees":["u"]}
{"board_id":"b","list_id":"l","card_id":"c","user_id":"u","kind":"member","action":"add","members":["u"],"assignees":[]}
```

Reads strictly decode the complete board/card document before projecting member
or assignee data. Board lists preserve inactive records and nullable permission
flags; card lists preserve selected user IDs in server order.

Board add/remove return summaries even for no-ops, including sole-admin removal.
Permission updates report Wekan's
affected-document count, including zero, and submitted input names in fixed
order. They do not claim that the requested flags were saved.

Every response object rejects unknown fields. Empty board mutation bodies,
invalid types, missing required fields, and mismatched card IDs produce
`protocol_error`. Embedded errors with HTTP 200 retain their Wekan status.
Transport, server, and response-validation failures report their actual errors. Mutations are never retried and no preflight or repair
request is sent. Human output escapes terminal controls and includes all mapped
fields, including nulls and empty arrays. Removals require no confirmation and
do not accept `--yes`.

## Examples

```console
wekan board member add USER_ID --board BOARD_ID --role normal
wekan board member update USER_ID --board BOARD_ID --role readonly
wekan --output json board member list --board BOARD_ID
wekan card member add USER_ID --board BOARD_ID --list LIST_ID --card CARD_ID
wekan card assignee remove USER_ID --board BOARD_ID --list LIST_ID --card CARD_ID
```

Board-member add/remove report request acceptance. An active-member addition can preserve permissions and sole-administrator removal can do nothing. Returned card membership snapshots and board permission-update counts retain their existing meaning.
