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

The `wekan user` family covers the complete Wekan v13.28 `/api/user`,
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

`take-ownership` transfers boards where the target user holds an administrator
membership. Wekan waits for the membership updates before returning the affected
board IDs and titles. If the caller is also the target, their membership can be
deactivated when the board has another active administrator.

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

Wekan allows an administrator to delete their own account only when the site
setting **Self delete user account** is enabled and another site administrator
exists. The default settings return HTTP 403 and retain the local credential.

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

For single-token revocation, supply the base64-encoded SHA-256 hash of the
issued token. Wekan acknowledges an unknown hash on an existing account. A raw bearer token is an unmatched value and receives the same acknowledgment.
An unknown account returns HTTP 404.

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

# Revoke this session using PowerShell 7.
$bytes = [Text.Encoding]::UTF8.GetBytes($issued.data.auth_token)
[Convert]::ToBase64String([Security.Cryptography.SHA256]::HashData($bytes)) |
    wekan user token delete $userId --token-stdin --yes

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
`login_disabled_reason`, `feature_profile_id`, `password_change_required`, `anonymized`, `anonymized_at`, `notification_delivery_revision`,
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
User lookup, update, ownership transfer and token deletion map HTTP 404 to `not_found`. If `user current` receives a valid user
document whose ID differs from the stored credential, it returns
`credential_store_failed` with exit status 6 so automation can repair the
credential. Invalid or rejected bearer credentials remain
`authentication_rejected`. If an HTTP-200 error object omits
`statusCode`, the CLI reports `protocol_error` while preserving any safe
`server_error`, `server_reason`, `server_message`, `server_error_type`, and
`server_is_client_safe` fields.

Creation returns the created user ID; deletion returns `deleted: true` after
the native endpoint confirms a matched user. Deleting a missing user returns
HTTP 404. Token creation returns the issued secret. Single-token deletion
acknowledges the supplied hash. Login disabling blocks REST sign-in.
