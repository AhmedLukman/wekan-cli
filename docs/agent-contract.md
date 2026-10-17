# Agent output contract

Use `--output json` for deterministic machine-readable output. Every envelope
is one JSON object followed by a newline.

## Streams

- Successful command envelopes are written to stdout and stderr is empty.
- Error envelopes are written to stderr and stdout is empty.
- `--help` and `--version` remain human-readable text on stdout.
- Human-mode errors are written to stderr.
- `api request --output raw`: a 2xx response body is written
  byte-for-byte to stdout; a non-2xx body is written byte-for-byte to stderr.
  Raw output adds no newline or envelope.
- Attachment downloads accept `--output raw` only with `--destination -`.
  The full typed response and base64 contents are validated first; decoded
  file bytes go to stdout with no envelope or newline. Errors are human-readable
  on stderr. A local stdout write failure can leave partial bytes.
- Interactive confirmation prompts are written to stderr. JSON output never
  prompts; destructive automation must pass command-local `--yes`.

## Response decoding diagnostics

Strict response-decoding failures retain the `protocol_error` code and exit
status 4. Their error details also include:

- `response_path`: a JSON Pointer-style location such as `/wipLimit/enabled`
  or `/0/title`. An empty string identifies the response root.
- `response_error`: `unknown_field`, `missing_field`, `invalid_type`,
  `invalid_value`, or `invalid_json`.

These diagnostics apply to typed successes, authentication responses, and
decoded Wekan error envelopes. For example:

```json
{"ok":false,"error":{"code":"protocol_error","message":"the list response contains unknown field at /wipLimit/future","details":{"response_path":"/wipLimit/future","response_error":"unknown_field","http_status":200}}}
```

Other context and mutation-outcome fields remain applicable. Diagnostics never
include rejected field values or the underlying deserializer's error text.
Field-name segments escape control characters and JSON Pointer metacharacters;
names longer than 80 characters and paths deeper than 16 segments are marked
`[truncated]`. Known secrets are redacted by the command's error mapper.
Semantic validation failures after decoding may omit these two fields.
Raw API output continues to preserve the upstream response without typed decoding.

## Destructive confirmation

`user token create` deliberately includes the issued `auth_token` secret in
successful JSON/human output. It returns no expiry and writes no local credential.
Its `creation_requested` is true and `mutation_confirmed` is false because Wekan
does not await token insertion, including when the target user does not exist.
`user token delete` redacts the supplied target token and authentication token
from its server message and error details. Its `scope` is `single` or `all`;
`deletion_requested` is true and `mutation_confirmed` is false. Debug output
redacts issued and supplied token material.

`auth logout` in every scope, `profile remove`, `user take-ownership`,
`user disable-login`, `user delete`, `user token delete` in both modes, `board delete`, `list delete`, `card delete`, `card bulk delete`,
`comment delete`, `checklist delete`, `checklist item delete`, `attachment delete`, and `swimlane delete`
require interactive confirmation or
`--yes`. Where a command performs target or removability preflight, it runs first and errors retain
their documented codes. Otherwise, non-terminal and JSON invocations without
`--yes` return `invalid_input` with exit status 2 before destructive work.
`--yes` is accepted only by those commands. For active-profile removal,
`--force` remains a separate requirement.

`api request` dynamically applies the same policy to every method except GET,
HEAD, and OPTIONS. Its cancellation operation is `api_request`; `--yes` is
rejected for its safe methods. Raw output may prompt when stdin and stderr are
terminals, while JSON output and non-terminal calls require `--yes`.

Logout holds the selected credential target's mutation guards while awaiting
interactive confirmation, binding approval to that target state and preventing
a concurrent Wekan CLI login or logout from mutating it before execution.

Declining an interactive prompt is an exit-0 no-op. Its structured result is:

```json
{
  "ok": true,
  "data": {
    "cancelled": true,
    "operation": "auth_logout"
  }
}
```

`operation` is `auth_logout`, `profile_remove`, `user_take_ownership`,
`user_disable_login`, `user_delete`, `user_token_delete`, `board_delete`, `list_delete`, `card_delete`, `card_bulk_delete`,
`comment_delete`, `checklist_delete`, `checklist_item_delete`, `attachment_delete`, `swimlane_delete`,
or `api_request`. Human output is
`Cancelled; no changes made.`

## Attachment results

Attachment results and transfer behavior are described in [attachments.md](attachments.md).
File open/read/staging/save failures use `file_io_error` and exit status 4.
If download completion reporting fails after the file was saved, error details
include `file_saved: true` and `destination`. Invalid transfer option combinations
use `invalid_input`, exit 2, before profile/credential access or HTTP.
Attachment mutations return `mutation_confirmed: false`; they acknowledge the
server's response without an authoritative readback. Optional output fields use
null when absent; response DTOs reject explicit null where the corrected contract
permits only omission. Scoped listings expose `requested_scope` and `effective_scope`.

## Export results

`board export`, `card export`, and `attachment export` use explicit destinations
and the shared [export contract](exports.md). File success metadata includes
resource/scoped IDs, format, destination, byte count, file-saved state, actual HTTP
status, and content type. Raw stdout requires `--destination -` and is emitted
only after the complete staged response validates; it contains original bytes.
JSON exports use strict mapped response objects, including nested activity and
rule data. They are not covered by the raw API's unmapped-field exception.
No aggregate 64 MiB export limit applies. Staging requires disk space for the
whole artifact, and JSON validation retains only the current record/string.
HTTP-200 native text failures are exit-5 protocol errors retaining status 200.
If completion reporting fails after saving, details retain `file_saved: true`
and `destination`.

## Raw API success

`api request` preserves arbitrary response data without typed Wekan decoding.
HTTP 2xx is successful even when the body contains a serialized Wekan error.
Human and JSON modes buffer at most 1 MiB; raw mode streams without that limit.

```json
{
  "ok": true,
  "data": {
    "http_status": 200,
    "mutation_attempted": false,
    "mutation_confirmed": false,
    "headers": [
      {"name": "content-type", "encoding": "text", "value": "application/json"}
    ],
    "body": {
      "encoding": "json",
      "value": {"unmodeled": {"fields": true}}
    }
  }
}
```

Header values use `text` or `base64`. Bodies use `json`, `text`, or `base64`.
Repeated response headers are separate entries. Non-2xx structured errors put
the same object in `error.details.response`; raw errors write only the exact
body to stderr and use the mapped nonzero exit status.

A structured response that cannot be read completely includes the known
`http_status` and `http_success`. An over-limit response additionally returns
`api_response_too_large` and includes `response_limit_bytes`. For an unsafe
request, `mutation_attempted` is `true` and `mutation_confirmed` is `false`:
the CLI performs no authoritative readback, so HTTP success means only that
Wekan accepted the request. If response streaming fails or a response is too
large, `outcome_unknown` is also `true` because the complete response was not
observed; do not retry the mutation automatically.

`--auth-token-query` is restricted to `GET`, but it can target any valid
same-origin raw API route. CLI-generated errors and debug output never include
its completed token-bearing URL. Upstream response headers and bodies remain
exact untrusted server data and can echo the URL or token.

## Public probes and board attachment links

`auth preflight <login|register|logout>` success data contains `server`,
`endpoint` (`login`, `register`, or `logout`), and `http_status: 200`. It
acknowledges a zero-byte HTTP 200 OPTIONS response; a nonempty successful body
is `protocol_error` with exit 4.

`system schema-upgrade-status` returns the complete typed state in the success
envelope, using snake_case fields and a step-name-ordered `steps` object. Failed
steps are reported as data and do not make a successful read exit nonzero.
See [system status](system.md) for nullable fields and step statuses.

Both public probes resolve a supplied server before a profile exists, without
creating profile metadata or accessing credentials. They support human/JSON
output. `board attachment list --board <ID>` requires stored authentication;
its data is `{board_id, count, attachments}` with the native legacy links and
nullable parent selectors. See [board attachment links](boards.md#attachment-links).

## Authentication success

Registration and login use the same secret-free data shape:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": "default",
    "user_id": "XQMZgynx9M79qTtQc",
    "token_expires": "2030-01-02T03:04:05Z",
    "credential_stored": true,
    "profile_created": true,
    "profile_active": true
  }
}
```

The `server` value is canonicalized and `profile` is always the selected profile
name. `profile_created` reports whether this command initialized that profile;
`profile_active` reports whether it is active after the command. The returned
token is absent because it is stored in the native credential store. Passwords
and two-factor codes are also never rendered.

Authentication status uses a nested allowlisted profile. Optional scalar
profile fields are always present and use `null` when Wekan omits them; missing
emails use an empty array, and optional email-entry fields also use `null`.

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": "default",
    "authenticated": true,
    "token_expires": "2030-01-02T03:04:05Z",
    "credential_stored": true,
    "user": {
      "user_id": "XQMZgynx9M79qTtQc",
      "username": "alice",
      "full_name": "Alice Example",
      "is_admin": false,
      "emails": [
        {
          "address": "alice@example.com",
          "verified": true
        }
      ]
    }
  }
}
```

Logout reports the completed scope and the resulting local credential state:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": "default",
    "logout_scope": "current_token",
    "remote_logout_completed": true,
    "credential_stored": false,
    "local_credential_removed": true
  }
}
```

`logout_scope` is `current_token`, `all_tokens`, or `local_only`. Local-only
success sets `remote_logout_completed: false`; it means only that the selected
local credential is absent, not that any Wekan token was revoked.
`local_credential_removed` distinguishes a deletion performed by this command
from an idempotent already-absent result. If remote logout succeeds but its
comparison observes a replacement, the newer record is preserved and success
sets `credential_stored: true` and `local_credential_removed: false`.

For example, a local-only deletion is:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": "default",
    "logout_scope": "local_only",
    "remote_logout_completed": false,
    "credential_stored": false,
    "local_credential_removed": true
  }
}
```

## User success

`user current`, `user get`, and no-action `user update` return the same typed, resource-focused user
shape. Optional scalar values are JSON `null`; collection fields are arrays.
The exact scalar allowlist is `user_id`, `username`, `full_name`, `emails`,
`is_admin`, `login_disabled`, `authentication_method`, `created_at`,
`modified_at`, and `last_connection_date`, plus typed organization, team, and
board-role memberships. The complete typed `profile` uses snake_case keys;
`created_through_api`, `heartbeat`, `session_data.total_hits`, and
`import_usernames` preserve decoded metadata. Password/token material and Wekan
services are excluded. Unknown response fields remain protocol errors.

List results are nested under their resource name: `{"users": [...]}` for
`user list`, `{"cards": [...]}` for `user cards`, and
`{"user_id": "...", "boards": [...]}` for `user boards`. A list entry's
`username` is nullable because Wekan permits email-only accounts.

Wekan v11.06's create-user response cannot provide the created ID:

```json
{
  "ok": true,
  "data": {
    "created": true,
    "username": "bob",
    "email": "bob@example.com",
    "user_id": null,
    "warning": "user_id_unavailable_in_wekan_v11_06"
  }
}
```

Login actions return `{"action":"disabled|enabled","user":{...}}`. In Wekan
v11.06, `disabled` records the requested action. Other-user requests set
`loginDisabled` and clear existing tokens; self-targeting is an unchanged-user
no-op. The returned user shows the actual state; the REST login route does not enforce that field and can still
mint a new token with correct credentials. After `enableLogin`, Wekan v11.06
omits `loginDisabled` from returned user documents, so the CLI outputs
`login_disabled: null`.
Ownership transfer returns `from_user_id`, `to_user_id`, and the affected
`boards` array. Deletion returns
`user_id`, `deleted`, `deleted_current_user`, `credential_stored`, and
`local_credential_removed`. The last three fields make self-delete cleanup
safe for automation.

## Board success

Board list data is `{"scope":"active|public","boards":[...]}`. Each board
summary contains only `board_id` and `title`. Active scope means the
authenticated user's active boards; public scope means the server-wide public
projection. Count data is `{"private":N,"public":N}` and represents
server-wide totals, not caller-visible totals.

`board get` returns the complete typed Wekan v11.06 board document with
snake_case names. `_id` is `board_id`, `backgroundImageURL` is
`background_image_url`, and `type` is `board_type`. Board ID and title are
required and non-empty. Every other known scalar is present as its value or
JSON `null`; every known collection is an array. Member, label, organization,
team, domain, and watcher entries are typed. An unmapped top-level or nested response
field causes `protocol_error` before the command produces success data.
Enum-constrained fields reject values outside the corrected v11.06 contract,
and timestamp fields reject values that are not RFC 3339 date-times.

Creation returns `{"board_id":"...","default_swimlane_id":"..."}`. Rename
returns `{"board_id":"...","title":"..."}`. Delete returns
`{"board_id":"...","deleted":true}` and verifies that Wekan returned the
requested ID. Declining board deletion returns the standard cancellation shape
with `operation: "board_delete"`.

## List success

`list list` returns `{"board_id":"...","lists":[...]}`. Each list summary
contains `list_id`, `title`, nullable `modified_at`, and nullable
`cards_modified_at`. Wekan v11.06 can include soft-deleted lists in this route
without exposing their deletion state; the CLI preserves the returned
collection and does not label it as active-only.

`list get` returns the complete typed Wekan v11.06 list document. `_id` is
`list_id`, `_updatedAt` is `position_updated_at`, and `type` is `list_type`.
Optional scalar fields are present as their value or JSON `null`, and the WIP
object is fully typed. The list type remains a string so values accepted by
Wekan are preserved. An empty color string is Wekan's valid persisted
unset-color state and is preserved. Unknown top-level or WIP fields, invalid
timestamps, nonempty colors, widths, and required empty identifiers cause
`protocol_error`.

Creation returns `{"board_id":"...","list_id":"..."}`. Update returns the
same identifiers plus a canonical-order `updated_fields` array containing any
of `title`, `color`, `starred`, and `wip_limit`. Delete returns
`{"board_id":"...","list_id":"...","deleted":true,"delete_mode":"soft"}`.
The delete result means Wekan accepted and validated the idempotent request; it
does not prove that the list previously existed or was newly deleted. Declining
list deletion returns the standard cancellation shape with
`operation: "list_delete"`.

## Card success

`card list --list` returns `{"board_id":"...","list_id":"...","cards":[...]}`.
Each projected card contains `card_id`, nullable `title`, `description`,
`swimlane_id`, `received_at`, `start_at`, `due_at`, and `end_at`, plus the
`assignees` array and nullable numeric `sort`.

`card list --swimlane` returns `{"board_id":"...","swimlane_id":"...","cards":[...]}`.
Its summaries project `list_id` instead of `swimlane_id`; `assignees` preserves
null arrays and null elements. The selectors conflict and exactly one is required.
`card count --board <ID> [--list <ID>]` returns
`{"board_id":"...","list_id":null,"count":N}`, with the selected list ID
instead of null for list counts. HTTP-200 serialized server errors remain errors.

`card get` returns the complete typed Wekan v11.06 card document. `_id` becomes
`card_id`, `type` becomes `card_type`, and every field uses snake case.
Omitted collections are empty arrays and omitted optional scalars are JSON
`null`. Custom-field values are the verified string, number, boolean, string
array with optional null members, or null union. Unknown top-level or nested
fields, invalid timestamps,
colors, card types, sticker highlights, identifiers, and nested value types
cause `protocol_error`.

`card get <ID>` uses ID-only lookup and can return archived cards. Supplying
both `--board` and `--list` selects the existing active-only scoped endpoint;
supplying just one parent is invalid. Both modes share the complete detail shape.

Creation returns `{"board_id":"...","list_id":"...","card_id":"..."}`.
Update returns those identifiers plus canonical-order `submitted_fields`, whose
possible values are `title`, `sort`, `parent_id`, `description`, `color`, `label_ids`,
`requested_by`, `assigned_by`, `received_at`, `start_at`, `due_at`, `end_at`,
`spent_time`, `is_over_time`, `members`, `assignees`, `due_complete`, `swimlane_id`,
`list_id`, `new_board_id`, `new_swimlane_id`, `new_list_id`, `archive`, `vote`,
`poker`, `custom_fields`, `stickers`, and `locations`. The returned board and list
IDs identify the request's source scope, including after movement.
These values report the wire fields sent; they do not claim Wekan persisted
them. A dispatched card update can persist earlier fields before a later field
fails, so such failures report `outcome_unknown: true` without claiming which
fields changed. Delete returns
`{"board_id":"...","list_id":"...","card_id":"...","deleted":true,"delete_mode":"hard"}`.
The delete result only means Wekan accepted its hard-idempotent request; it does
not prove prior existence or removal. Declining card deletion uses the standard
cancellation shape with `operation: "card_delete"`.

`card archive` returns
`{"board_id":"...","list_id":"...","card_id":"...","archived":true,"archived_at":"<RFC3339>"}`.
`card restore` returns the same identifiers and `"archived":false`, without
`archived_at`. Archive's timestamp comes from the acknowledgement and can
differ from the document timestamp. Restore keeps the stored timestamp. Both
commands recursively change child cards and run without confirmation.

These dedicated commands require the actual board/list/card scope and expected
initial archived state. Unknown fields, missing or invalid required values,
unexpected result states, and mismatched card IDs produce `protocol_error`
with `outcome_unknown:true`. Wekan v11.06 missing-card, wrong-list, wrong-existing-board,
and wrong-state failures are HTTP 500 HTML responses; they remain `server_error`
with `http_status:500` and `outcome_unknown:true`. Missing boards produce
`not_found` with HTTP 404. HTTP 401/403 retains authentication/permission
classification. No command preflight, retry, or follow-up request is issued.

## Copy and move success

`board copy` returns `source_board_id` and the new `board_id`. `card copy`
returns `source_board_id`, `source_list_id`, `source_card_id`, requested
`target_board_id`, `target_list_id`, `target_swimlane_id`, and new `card_id`.
Omitted destination board/list values are resolved to the source selectors.
Both results include `mutation_confirmed: false`, and human output states that
effects are unconfirmed: Wekan does not await attachment duplication, which fails
independently of the returned ID in v11.06.

List transfers return `source_board_id`, `source_list_id`, `target_board_id`,
nullable `target_swimlane_id`, `returned_list_id`, and `mutation_confirmed: false`.
Swimlane transfers return `source_board_id`, `source_swimlane_id`,
`target_board_id`, `returned_swimlane_id`, and `mutation_confirmed: false`.
List moves can return the source ID while retaining that list. These results
acknowledge native requests without independently verifying their effects.
See [copy and move](copy-and-move.md) for defaults, ordering, and pinned defects.

## Comment success

`comment list` returns `{"board_id":"...","card_id":"...","comments":[...]}`.
Each compact comment contains `comment_id`, `text`, and `author_id`.

`comment get` returns `comment_id`, `board_id`, `card_id`, `text`, nullable
`parent_id`, `created_at`, `modified_at`, and `author_id`. Empty, null, or
omitted top-level parent IDs become JSON `null`. Unknown response fields,
missing or empty required values, and invalid timestamps cause
`protocol_error`. Wekan v11.06 returns an empty HTTP 200 body for a missing
comment; the CLI normalizes that response to `not_found`.

Creation returns `{"board_id":"...","card_id":"...","comment_id":"..."}`.
Delete returns the same identifiers plus `"deleted":true` and
`"delete_mode":"hard"`. A missing board or comment produces `not_found` and is
known not to have deleted anything. A malformed successful response or a
returned card ID that differs from the request reports `outcome_unknown: true`.
Declining comment deletion uses the standard cancellation shape with
`operation: "comment_delete"`.

## Checklist and checklist item success

`checklist get` and `checklist item get` return complete typed documents with
`checklist_id` or `item_id`, respectively. Both detail results include optional
`user_id` creator metadata: Wekan UI/DDP-created records populate it, while
REST-created records omit it. Missing creator metadata is null in JSON output.
The CLI reads both kinds through REST. Nested checklist item summaries contain
only `item_id`, `title`, and `is_finished`.
Detail `board_id` preserves Wekan's denormalized metadata. After a REST card move
to another board, it may still identify the source board while requests must use
the destination board selector.

Collections include requested board/card scope; mutation results include scope
and returned IDs. Item updates report deterministic `submitted_fields`, ordered
`title` then `is_finished`, or an empty array for an empty update. Deletions
return `deleted: true` and `delete_mode: "hard"`. See
[checklist commands](checklists.md#stable-output) for every mapped detail field.

## Swimlane success

`swimlane list` returns `{"board_id":"...","swimlanes":[...]}`. Each
summary contains only `swimlane_id` and `title`.

`swimlane get` returns the complete typed Wekan v11.06 swimlane document.
`_id` is `swimlane_id`, and `type` is `swimlane_type`. Optional archive and
update timestamps, sort, color, and height are present as their value or JSON
`null`. Unknown fields, invalid timestamps or colors, heights outside `-1` or
50 through 2000, and required empty identifiers cause `protocol_error`.

Creation returns `{"board_id":"...","swimlane_id":"..."}`. Update returns
the same identifiers plus `"updated_fields":["title"]`. Delete returns
`{"board_id":"...","swimlane_id":"...","deleted":true,"delete_mode":"hard"}`.
The delete result means Wekan accepted and validated the idempotent request; it
does not prove that the swimlane previously existed or was newly deleted.
Declining swimlane deletion returns the standard cancellation shape with
`operation: "swimlane_delete"`.

## Profile success

Profile objects use a stable `name`, canonical `server`, and boolean `active`
shape. A list result contains the nullable active name and name-sorted profile
array:

```json
{
  "ok": true,
  "data": {
    "active_profile": "work",
    "profiles": [
      {
        "name": "personal",
        "server": "https://wekan.example/",
        "active": false
      },
      {
        "name": "work",
        "server": "https://work.example/",
        "active": true
      }
    ]
  }
}
```

An empty store uses `active_profile: null` and `profiles: []`.

`profile add`, `show`, `use`, and `update` return one profile object as their
data. Removal reports both what was removed and the resulting selection:

```json
{
  "ok": true,
  "data": {
    "name": "work",
    "server": "https://work.example/",
    "removed": true,
    "active_profile": null
  }
}
```

## Errors

```json
{
  "ok": false,
  "error": {
    "code": "login_rate_limited",
    "message": "too many failed login attempts; try again later; retry after 30 seconds",
    "details": {
      "profile": "default",
      "http_status": 429,
      "server_error": "too-many-requests",
      "retry_after_seconds": 30
    }
  }
}
```

`details` is always an object. Depending on the error it can contain:

- `http_status`: HTTP status returned by the server;
- `http_success`: whether a raw API response status was in the 2xx range when
  its complete structured representation could not be buffered;
- `response_limit_bytes`: the applicable structured response-size limit;
- `retry_safe`: whether retrying after a local output failure is safe; it is
  `false` for an unsafe raw request whose HTTP success was already received;
- `mutation_attempted`: whether an unsafe raw request received an HTTP response;
- `mutation_confirmed`: whether the CLI performed an authoritative readback and
  confirmed the mutation; raw API requests currently report `false`;
- `wekan_status_code`: application status serialized inside a Wekan response;
- `server_error`, `server_reason`, and `server_message`: structured Wekan error
  fields;
- `server_error_type`: the serialized Wekan/Meteor error type, when present;
- `server_is_client_safe`: whether Wekan marked the serialized error safe for
  clients;
- `account_created`: whether account creation is known to have occurred;
- `session_created`: whether login session creation is known to have occurred;
- `two_factor_required`: whether Wekan accepted the password but requires a
  two-factor code for login;
- `outcome_unknown`: whether the server may have processed a failed request;
- `retry_after_seconds`: a valid integer `Retry-After` value from HTTP 429;
- `token_expires`: the stored RFC 3339 expiry when a credential is locally
  expired;
- `logout_scope`: the requested current-token, all-token, or local-only
  operation when relevant;
- `remote_logout_completed`: `true` after a validated logout response, `false`
  when remote logout conclusively did not run, or `null` when its outcome cannot
  be validated;
- `credential_stored`: the known local credential presence at the guarded
  command outcome;
- `local_credential_removed`: whether this invocation is known to have removed
  the local credential;
- `user_deleted`: whether remote user deletion is known to have completed when
  a later local cleanup step fails;
- `profile_created`: whether this command created the selected profile;
- `profile_active`: whether that profile is active after the known local
  outcome;
- `profile`: the selected profile associated with the error, when target
  context is relevant.
- `response`: the complete status, headers, and tagged body returned by a raw
  API request when structured output could buffer it.

Registration HTTP 400 errors always set `outcome_unknown: true` because Wekan
`v11.06` can return that status before or after account creation. Authentication
transport failures and HTTP 5xx responses also set it because an intermediary
can fail after forwarding the request.

An invalid, unreadable, or oversized HTTP 200 registration response sets
`account_created: true`; the login equivalent sets `session_created: true`.
A vault write failure uses the same operation-specific field because it happens
after a validated success response.

A profile-write error distinguishes whether the new document was installed.
Errors before installation report `profile_created: false`. If installation
completed but final synchronization or inspection failed, the error reports
`profile_created: true` and the resulting `profile_active` state. Neither path
writes the credential.

Registration and login serialize their raw-entry checks and credential creation
with current Wekan CLI processes. When the CLI observes a raw entry in the
selected profile account before secret input or HTTP, it returns
`credential_already_exists` without decoding or overwriting that observed
entry. A conflict the CLI observes after remote success includes
`account_created: true` or `session_created: true` plus the known
`profile_created`, `profile_active`, and `credential_stored` state. Generic OS
keyring APIs do not provide a cross-process atomic create or compare-and-swap
operation, so a non-cooperating external vault writer can still race between
the CLI's check and its write.
`profile_server_mismatch` means a supplied URL did not match the selected
existing profile; resolution stops before credential access.

Login HTTP 400 is a malformed-request `protocol_error`. HTTP 401 is
`login_rejected`; when Wekan specifically returns `no-2fa-code`, details include
`two_factor_required: true` so automation can request a code without depending
on the upstream error string. The CLI never retries that request automatically.

Status uses `credential_not_found` when the resolved target has no saved record,
`credential_expired` when the saved expiry is not in the future, and
`authentication_rejected` when Wekan rejects the bearer token. Wekan v11.06's
current-user route serializes rejection inside HTTP 200, so details contain
both the actual `http_status` and the embedded `wekan_status_code`. Status is
read-only and never removes the offending record. Since login will not
overwrite any stored entry, callers must remove an expired or rejected entry
with `auth logout --local-only` before logging in again.

Normal logout also uses `credential_not_found` when no record can be submitted.
It preserves the record on every remote error. Transport failures, redirects,
and 5xx responses set `outcome_unknown: true` and
`remote_logout_completed: null`; an unusable HTTP 200 sets both
`remote_logout_completed: null` and `outcome_unknown: true` and remains a
`protocol_error`. Local-only logout is idempotent for an absent entry and never
contacts Wekan. Logout mutation requests are never automatically retried.

The relevant details for an unusable current-token HTTP 200 are:

```json
{
  "profile": "default",
  "http_status": 200,
  "outcome_unknown": true,
  "logout_scope": "current_token",
  "remote_logout_completed": null,
  "credential_stored": true,
  "local_credential_removed": false
}
```

Messages are intended for humans. Automation must branch on `error.code` and
the process exit status. Stable error codes are:

- `invalid_input`
- `configuration_error`
- `insecure_transport`
- `profile_not_found`
- `profile_already_exists`
- `profile_server_mismatch`
- `profile_in_use`
- `profile_has_credential`
- `transport_error`
- `unexpected_redirect`
- `protocol_error`
- `api_response_too_large`
- `login_rejected`
- `login_rate_limited`
- `credential_not_found`
- `credential_expired`
- `credential_already_exists`
- `authentication_rejected`
- `permission_denied`
- `not_found`
- `registration_rejected`
- `registration_disabled`
- `server_error`
- `credential_store_unavailable`
- `credential_store_failed`
- `internal_error`

Passwords, two-factor codes, and managed tokens are redacted from all
CLI-generated fields. Exact raw API response headers and bodies are upstream
data and are not redacted.

The profile conflict codes and `credential_already_exists` use exit status 3.
Malformed, unsupported, unreadable, or unwritable profile storage uses
`configuration_error`; native vault failures retain the credential error family
and exit status 6.

## Exit statuses

| Exit | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Unexpected internal failure |
| `2` | CLI usage, missing/ambiguous identity, empty secret, or registration password mismatch |
| `3` | Missing or invalid server/profile configuration, profile conflicts, or insecure transport refusal |
| `4` | Transport, redirect, protocol, or local response-output failure, including `api_response_too_large` |
| `5` | Unauthenticated state, authentication rejection, or another Wekan application/server rejection |
| `6` | Credential store unavailable or failed, including a created account or session whose token could not be stored |

When a non-success error response body is empty or is not JSON, its HTTP status
still determines the error classification and exit status. A syntactically
valid JSON error object with unmapped fields or invalid known-field types is a
`protocol_error`; it is never silently reduced to a generic server error.

## Memberships

Nested `board member`, `card member`, and `card assignee` commands use the
standard human/JSON envelopes. Board lists preserve all membership records,
including inactive members and nullable optional permission flags. Card lists
preserve the selected user-ID array in server order. These reads strictly
decode the complete board/card document before projecting the membership data.

Board add/remove expose `board_id`, `user_id`, `action`, returned `boards`, and
`mutation_confirmed: false`. Permission updates expose `board_id`, `user_id`,
`affected_documents`, and ordered `submitted_fields`; counts are not saved
permission values. Card mutations expose `board_id`, `list_id`, `card_id`,
`user_id`, `kind` (`member` or `assignee`), `action` (`add` or `remove`), and both
returned `members` and `assignees` arrays from the requested card. For linked
cards, Wekan changes the source card but returns the link's stored arrays;
those arrays do not confirm the source's saved assignments. Unknown fields,
invalid response types, empty board mutation bodies, and mismatched card IDs
fail explicitly.
Ambiguous mutation errors include `outcome_unknown: true`; no retry or repair
occurs. Removals require no confirmation and accept no `--yes`.
See [membership commands](memberships.md) for permissions and pinned no-ops.

## Custom fields

Custom-field and dropdown-item commands use the standard typed success/error
envelopes and human/JSON formats. Definitions expose snake-case IDs, field types,
settings and display controls; nested dropdown items expose item_id/name.
Optional settings accept explicit nulls, preserve them in requests, and expose
both absent and null properties as null in stable output.
Creation acknowledges a field ID, including dropdown creation; updates expose
ordered submitted_fields. Card assignment returns its complete cleaned field
array; lookup returns full card details plus the exact string query and scope.

All response objects reject unmapped top-level and nested fields. Malformed or
mismatched mutation acknowledgments report protocol_error with an unknown
outcome. The two deletion commands require confirmation or --yes; cancellation
operations are custom_field_delete and custom_field_item_delete. Success exposes
deletion_requested: true and mutation_confirmed: false, including successful
no-ops; it does not claim completed removal. Shared definition deletion affects
all associated boards and their card values. Dropdown-item deletion changes the
shared definition's options for every associated board. Both confirmation
prompts disclose the full scope and identify the requested board.

Exact JSON input is sent without coercion; Wekan cleans card values before
returning them. Omitted assignment value defaults to an empty string; explicit
null remains null. REST writes can bypass UI hooks and definition updates can
fail after earlier writes. No automatic retry or repair occurs. See
[custom-field commands and pinned behavior](custom-fields.md) for all options,
input conflicts, stable fields, and server limitations.

Missing definition lookup maps an empty HTTP 200 to not_found with http_status
200 and no wekan_status_code, server_error or server_reason. Lookup rejects empty
and dot-only (`.` or `..`) values as invalid_input before credentials or HTTP;
the URL library cannot preserve these dot-only segments.
The custom-field families and the two card custom-field commands likewise
reject dot-only target IDs before credentials or HTTP. Supplied settings reject
dropdown item IDs that become dot-only after Wekan's JavaScript trimming;
existing responses containing these IDs still decode normally.
Target parsing and response ID validation use the same JavaScript whitespace
rules: U+0085 is preserved, including IDs consisting only of that character,
while U+FEFF is trimmed. Distinct dropdown IDs remain distinct mutation targets.

## Bulk cards and labels

Complete batches use ok: true with the typed result in data. Any creation item
error or nonempty not_found returns batch_incomplete, exit 5, with empty stdout
and the error envelope on stderr. error.details.batch_result contains operation
(card_bulk_create, card_bulk_delete, or card_bulk_labels) and data holding the
complete result, including successes. Result shapes match successful batches.
HTTP metadata retains http_status: 200 and http_success: true; retry_safe: false
prevents treating the whole batch as safe to repeat. Human errors include each
outcome and a count summary.

Creation results expose board_id, list_id, and results; each item has index,
card_id, and error, with the unused outcome represented as null. Deletion exposes
board_id, deleted, and not_found. Label changes expose board_id, updated (objects
with card_id and label_ids), and not_found. Order and duplicate records are
preserved. Bulk label_ids arrays preserve nullable elements, including in
batch_incomplete results. Mutation protocol/transport failures retain
outcome_unknown: true; a server failure can follow earlier writes. No automatic
retry or readback occurs.

Invalid added labels preserve HTTP 400 server_error plus invalid_label_ids
through an endpoint-specific strict decoder. File/stdin read failures use
file_io_error, exit 4; invalid JSON, unsupported properties, invalid field types
and batch sizes use invalid_input, exit 2.

Board label addition returns board_id and label_id because Wekan may reuse an
existing label. Board label lists expose board_id and labels (label_id, nullable
name, color). Card label lists expose board_id, list_id, card_id, and label_ids,
preserving null elements. Both strictly decode the complete source document.
Bulk deletion reloads and validates the guarded credential after confirmation;
replacement, removal, or expiry prevents HTTP and preserves the vault's current
state.
Bulk deletion's `deleted` array confirms card deletion only. Wekan v11.06
retains ordinary attachment metadata and file bytes; successful batches do not
confirm attachment cleanup. The CLI performs no additional cleanup requests.
See [bulk card and label behavior](bulk-cards-and-labels.md).


## Board automation rules

`rule list/get/create/update/delete` uses typed human/JSON results. List data is
`{board_id,rules}`; each detail includes `board_id`, `rule_id`, `title`, and
nullable `trigger`/`action`. Nested fields use snake_case, omit absent fields,
and preserve explicit nulls. Unknown fields at the envelope or embedded object
boundary fail with `protocol_error`; no opaque maps bypass mapping.

Creation returns `board_id`, `rule_id`, `trigger_id`, and `action_id`. Update
returns `board_id`, `rule_id`, `submitted_fields` (title/trigger/action order),
and `mutation_confirmed: false`. Deletion returns `board_id`, `rule_id`, and
`deleted: true`; cancellation uses operation `rule_delete`.

Create/update require exactly one `--rule`, `--rule-file`, or `--rule-stdin`.
Typed camelCase JSON input is limited to 50 MiB. Supplied `trigger.lastRunAt`
values must be valid Wekan date-time strings or null; invalid values fail with
`invalid_input` before credentials or HTTP. Get/list responses are bounded to
64 MiB; exceeding this aggregate limit returns `protocol_error` (exit 4).
Sparse updates, including `{}`, are passed to native shallow merging.
Only deletion accepts `--yes`.
Missing rules retain HTTP 500 `not-found` as exit-5 `server_error`; missing boards
retain HTTP 404. Mutation errors can indicate `outcome_unknown: true`; no retry,
rollback, or readback is performed. See [rule behavior](rules.md).

## Global administrator settings

`admin settings get/update` and `admin attachment-settings get/update` return
`data.settings` with snake_case keys, including `settings_id` and all mapped
optional fields. Updates add canonical-order `submitted_fields`, using top-level
snake_case names. Cloud secrets are excluded; boolean presence markers remain.
The response is stored configuration, not a connectivity or migration result.

Each command sends one native request. Nested section replacement/defaults and
cloud-provider merging remain native. Strict decoding rejects unmapped fields
at every object boundary, invalid known-field types/dates, and unmasked secrets.
HTTP-200 application errors retain native metadata; failed updates retain
mutation-outcome uncertainty. See [administrator settings](admin-settings.md)
for complete input, credential, and replacement semantics.

## Import results

`board import` returns `board_id`, `source` (`native` when --source is omitted),
and `mutation_confirmed: false`. ZIP results include the destination board,
requested nullable scope IDs, all six native counters under `counts`, and
`mutation_confirmed: false`. ICS results include destination board/list/swimlane,
`created`, `card_ids`, and `mutation_confirmed: false`. These are acknowledgements
without readback. Only human and JSON output are supported; imports do not prompt.
Strict response and embedded-error decoding applies, including nested ZIP counts.
Timeouts and invalid success bodies can follow partial writes; errors preserve
`outcome_unknown: true` where applicable. See [imports](imports.md).
