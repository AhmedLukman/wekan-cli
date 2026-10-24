# Users

Profile dialog preferences (`move_and_copy_dialog`, `move_checklist_dialog`, and
`copy_checklist_dialog`) are dictionaries keyed by the source board ID. Each
value has required string `board_id`, `swimlane_id`, and `list_id` fields, plus
nullable `card_id` (null when absent). Empty selection strings are preserved.
Unknown nested fields and invalid types produce `protocol_error`, including
when the profile is decoded for `auth status`. `list_constraints` retains
dynamic board/list IDs with numeric values; fractions, zero, and negative
numbers are preserved without applying anonymous-browser storage limits.

Profile notifications expose `activity`, nullable `read`, and nullable `show_at`.
`show_at` preserves Wekan's numeric milliseconds since Unix epoch for scheduled
tray delivery or quiet hours. It is null when Wekan omits `showAt` for immediate
delivery without quiet hours. An explicit upstream null or a nonnumeric `showAt`
is a protocol error.

The `wekan user` family covers the complete Wekan v12.57 `/api/user`,
`/api/users`, and API token REST slice: ten operations across seven paths. Every command
requires an existing profile and a stored credential whose expiry is still in the future.

```text
wekan user current
wekan user cards [--due] [--from <DATE>] [--to <DATE>]
wekan user list
wekan user get <USER>
wekan user update <USER_ID>
wekan user create [--username <NAME>] [--email <EMAIL>] [--password-stdin | --password-raw-stdin | --password-digest-stdin]
wekan user boards <USER_ID>
wekan user take-ownership <USER_ID> [--yes]
wekan user disable-login <USER_ID> [--yes]
wekan user enable-login <USER_ID>
wekan user delete <USER_ID> [--yes]
wekan user token create <USER_ID> --reason <TEXT>
wekan user token delete <USER_ID> (--token-stdin | --all) [--yes]
```

`current` calls `GET /api/user`. Its exact allowlist is `_id`, `username`,
emails (`address`, `verified`), the complete typed `profile`, `isAdmin`, `loginDisabled`,
`authenticationMethod`, `createdAt`, `modifiedAt`, `createdThroughApi`, `heartbeat`, `sessionData.totalHits`,
`importUsernames`, `lastConnectionDate`, typed
organization/team memberships, and typed board-role flags. The shared client
operation also powers
`auth status`, but the commands intentionally project it differently:
`user current` is resource-focused, while `auth status` adds local profile,
credential-presence, and token-expiry metadata.

`cards` calls `GET /api/user/cards`. Wekan returns non-archived cards on active
boards readable by the caller where the caller is a card member, assignee,
requester, or assigner. `--due` requires a due date. `--from` and `--to` accept
timestamps, date-only and textual dates interpreted by Wekan. Either bound
enables due-date filtering; a reversed valid range returns an empty array. Missing or
invalid authentication returns HTTP 401 on both this route and `GET /api/user`.

`list`, `get`, `update`, `create`, `take-ownership`, `disable-login`, `enable-login`,
`delete`, and both `token` commands require a Wekan site administrator. `get` accepts a user ID or
username. `boards` requires either the selected user's own credential or a site
administrator and returns active, non-archived, non-internal board ID/title
summaries. The authenticated-self form is also exposed as `wekan board list`.
Wekan usernames are optional, so `list` emits JSON `null` and the
human table displays `<none>` for an email-only account.

`update <USER_ID>` sends an empty PUT body, exercising Wekan's no-action form
and returning the unchanged user. Use the dedicated commands for actual actions.

## Creating users

The new password is never accepted as a command-line value or environment
variable. Interactive use reads and confirms it without echo. Automation uses
`--password-stdin` and supplies exactly one password line. The password and
stored bearer token are redacted from every failure and never appear in output.
`--password-raw-stdin` instead preserves all UTF-8 input through EOF, including
line breaks. `--password-digest-stdin` reads a JSON object with a 64-character
`digest` and `algorithm: "sha-256"` through EOF. These three stdin modes conflict;
passwords and digests are never command-line values.

Supply at least one nonempty `--username` or `--email`. Both may be supplied;
an omitted complement is sent as an explicit empty string, as required by the
pinned route for username-only and email-only accounts.

Successful creation reports the supplied username, email, and the created user ID:

```json
{
  "ok": true,
  "data": {
    "username": "bob",
    "email": "bob@example.com",
    "user_id": "created-user-id"
  }
}
```

## Administrative actions and safety

`take-ownership` sends Wekan's explicit `takeOwnership` action and returns
selected board summaries. Wekan v12.57 selects boards containing the target
user's membership and any administrator membership; those can be different
members. The target can therefore be inactive or a non-administrator. The
server replaces that membership's user ID with the caller's ID, preserving its
flags, and can leave duplicate caller memberships. Returned summaries do not
confirm completed transfers: Wekan starts the changes without awaiting them.
Inspect membership readback before relying on the result.

Self-targeting is sent after confirmation. When
another active board administrator exists, Wekan v12.57 can deactivate the
caller's membership; the CLI preserves that upstream defect.

`disable-login` sends `disableLogin`, sets the target's `loginDisabled` field,
and clears their existing Wekan sessions. Self-targeting is sent after
confirmation; Wekan's pinned route returns the unchanged user and does not
disable the caller. Disabled accounts cannot start new REST sessions.
`enable-login` sends `enableLogin` and needs no confirmation. It returns
`login_disabled: false` after enabling the account.

Ownership transfer, login disabling, and deletion prompt interactively and
default to no. Non-interactive and JSON invocations must pass command-local
`--yes`. Declining returns an exit-0 cancellation and sends no mutation request.
Cancellation uses `user_take_ownership`, `user_disable_login`, or `user_delete`
for those respective commands. Token-deletion cancellation uses
`user_token_delete`.
Mutation requests are bounded, never follow redirects, and are never retried.
Transport failures, redirects, server failures, and unusable successful bodies
report their actual error codes and diagnostics.

Deleting the authenticated caller acquires the same profile/server credential
mutation guards used by authentication mutations and holds them through remote
success and cleanup. It conditionally removes only the credential record used
for the request. A replacement observed before the request aborts without HTTP;
a mismatching replacement is never deleted. If Wekan deletes the current user
but local credential removal fails, the structured error reports
`user_deleted: true` and `credential_stored: true` so automation can distinguish
that partial result from a failure before the HTTP request.

## API tokens

`user token create <USER_ID> --reason <TEXT>` sends one JSON `POST /api/createtoken/{user}` containing the required administrative audit reason. Wekan rejects unknown users with HTTP 404 before issuing a token.
It returns `user_id` and `auth_token` in the standard success envelope. Human
output reports that Wekan returned a token and prints the secret. There is no returned
expiry, token listing, token import, or automatic credential storage. The request
uses the selected profile's existing credential; the target must be a user ID,
not a username.

`user token delete <USER_ID>` requires exactly one of `--token-stdin` or `--all`.
Single-token input is UTF-8 through EOF, with one final LF or CRLF removed;
other whitespace is preserved. Empty input fails before HTTP. There is no
token argument or environment-variable input. Both deletion modes require
confirmation or `--yes`; JSON and piped invocations require `--yes`.

Deletion sends one JSON `POST /api/deletetoken`: `{userId, token}` for a supplied
token or `{userId}` for all tokens. Results contain `user_id`, `scope` (`single`
or `all`), and `message`.
The server's message can contain the target secret; the CLI redacts both that
secret and its authentication credential in the message and in error details.
Debug output never reveals tokens. Human rendering escapes terminal controls.

Single-token deletion sends the supplied token unchanged. W11-058 remains:
Wekan acknowledges a raw issued token without revoking it because its account
API compares that value directly with stored hashes. Other sessions remain
valid. Nonexistent tokens and users also receive an acknowledgment. The CLI
does not hash, retry, or substitute a delete-all request.

Delete-all revokes every existing session for the selected user, including the
caller's own session when targeting itself. Live readback confirms the saved
token array is empty; ordinary password login can create a new session afterward.
These commands retain profiles and local vault entries. To remove a credential
that was revoked remotely, run `wekan auth logout --local-only --yes`, then log in.
Self-targeted delete-all holds the selected credential's mutation lock through
confirmation and HTTP. A replaced, removed, or expired credential stops the
request before HTTP; concurrent local logout/login waits until the request ends.

Examples (PowerShell):

```powershell
# This success output contains the issued secret.
$issued = wekan --output json user token create $userId --reason "Automation access" | ConvertFrom-Json
$issued.data.auth_token

# Request deletion of the raw issued token; Wekan v12.57 acknowledges it
# without revoking it (W11-058).
$issued.data.auth_token | wekan user token delete $userId --token-stdin --yes

# Revoke all existing sessions for the target user.
wekan user token delete $userId --all --yes
```

## Output and errors

Lists use compact human tables. Detail and mutation commands use labelled
fields. Every string received from Wekan is escaped before terminal rendering.
JSON uses the standard `{ "ok": true, "data": ... }` envelope.

`user current`, `user get`, and no-action `user update` return the same typed,
resource-focused shape. Stable scalar/output names include `user_id`, `username`,
`full_name`, `emails`, `is_admin`, `login_disabled`, `authentication_method`,
`created_at`, `modified_at`, and `last_connection_date`, plus typed organization,
team, and board-role memberships. The complete `profile` is always an object
with snake_case keys. `created_through_api`, `heartbeat`, `feature_preview`, `inactivity_clock_start`,
`login_disabled_reason`, `anonymized`, `anonymized_at`, `notification_delivery_revision`,
`session_data.total_hits`, and `import_usernames` retain mapped metadata.
Board-role memberships include `granted_by` group grants and `custom_role_id`;
both are null when absent.

Optional scalars are null. Top-level collections are arrays; optional arrays
and maps inside `profile`, including workspace `children`, are null when absent.
Workspace nodes map `id`, `name`, `icon`, and recursive `children`; additional
fields on a node or child produce `protocol_error`. Services and password or
token material are excluded. Unmapped fields are rejected rather than omitted.

Collection data is `{"users":[...]}` for `user list`, `{"cards":[...]}` for
`user cards`, and `{"user_id":"...","boards":[...]}` for `user boards`.
Login actions return `{"action":"disabled|enabled","user":{...}}`.
Ownership transfer returns `from_user_id`, `to_user_id`, and the affected `boards`.
Deletion returns `user_id`, `deleted`, `deleted_current_user`,
`credential_stored`, and `local_credential_removed`; the last three describe
self-delete cleanup.
Verified HTTP or embedded status 403 maps to `permission_denied`.
The user operations do not consistently expose a verified not-found identifier,
so other server and embedded errors preserve their original metadata instead of
being generalized to `user_not_found`. If `user current` receives a valid user
document whose ID differs from the stored credential, it returns
`credential_store_failed` with exit status 6 so automation can repair the
credential. Invalid or rejected bearer credentials remain
`authentication_rejected`. If an HTTP-200 error object omits
`statusCode`, the CLI reports `protocol_error` while preserving any safe
`server_error`, `server_reason`, `server_message`, `server_error_type`, and
`server_is_client_safe` fields.

Creation returns the created user ID; deletion returns `deleted: true` after
the native endpoint confirms a matched user. Deleting a missing user returns
HTTP 404. Missing-user lookup still returns safe HTTP 500 and `server_error`;
an update against a missing user returns an empty HTTP 200 body, which strict
typed decoding reports as `protocol_error` with the native status retained
(W11-015). These responses are not generalized to a not-found result.
Token creation returns the issued secret; single-token deletion reports request
acceptance and explains W11-058. Login disabling now blocks REST sign-in.
Ownership transfer reports a requested action because the native ownership
change remains unawaited.
