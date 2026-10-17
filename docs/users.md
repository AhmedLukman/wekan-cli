# Users

The `wekan user` family covers the complete Wekan v11.06 `/api/user`,
`/api/users`, and API token REST slice: ten operations across seven paths. Every command
requires an existing profile and a stored credential whose expiry is still in the future.

```text
wekan user current
wekan user cards [--due] [--from <DATE>] [--to <DATE>]
wekan user list
wekan user get <USER>
wekan user update <USER_ID>
wekan user create --username <NAME> --email <EMAIL> [--password-stdin | --password-raw-stdin | --password-digest-stdin]
wekan user boards <USER_ID>
wekan user take-ownership <USER_ID> [--yes]
wekan user disable-login <USER_ID> [--yes]
wekan user enable-login <USER_ID>
wekan user delete <USER_ID> [--yes]
wekan user token create <USER_ID>
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
invalid authentication is an HTTP 401 response on this route, unlike the
HTTP-200 embedded authentication error returned by `GET /api/user`.

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

Wekan v11.06 creates the account but returns `_id: {}` because its handler
serializes an unawaited `Accounts.createUser` Promise. The CLI does not perform
an ambiguous username lookup afterward. It reports:

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

## Administrative actions and safety

`take-ownership` sends the explicit `takeOwnership` action. It transfers every
board administered by the target user to the authenticated caller and returns
the affected board summaries. Self-targeting is sent after confirmation. When
another active board administrator exists, Wekan v11.06 can deactivate the
caller's membership; the CLI preserves that upstream defect.

`disable-login` sends `disableLogin`, sets the target's `loginDisabled` field,
and clears their existing Wekan sessions. Self-targeting is sent after
confirmation; Wekan's pinned route returns the unchanged user and does not
disable the caller. Wekan v11.06's REST login handler does not inspect
`loginDisabled`, so
correct credentials can still create a new REST session; this is documented as
an upstream defect rather than reported as confirmed access prevention.
`enable-login` sends `enableLogin` and needs no confirmation. Wekan v11.06
omits `loginDisabled` from the returned and later user documents after enabling
login, so the CLI outputs `login_disabled: null`.

Ownership transfer, login disabling, and deletion prompt interactively and
default to no. Non-interactive and JSON invocations must pass command-local
`--yes`. Declining returns an exit-0 cancellation and sends no mutation request.
Mutation requests are bounded, never follow redirects, and are never retried.
Transport failures, redirects, server failures, and unusable successful bodies
report `outcome_unknown: true` when Wekan may have applied the mutation.

Deleting the authenticated caller acquires the same profile/server credential
mutation guards used by authentication mutations and holds them through remote
success and cleanup. It conditionally removes only the credential record used
for the request. A replacement observed before the request aborts without HTTP;
a mismatching replacement is never deleted. If Wekan deletes the current user
but local credential removal fails, the structured error reports
`user_deleted: true` and `credential_stored: true` so automation can distinguish
that partial result from a failure before the HTTP request.

## API tokens

`user token create <USER_ID>` sends one bodyless `POST /api/createtoken/{user}`.
It returns `user_id`, `auth_token`, `creation_requested: true`, and
`mutation_confirmed: false` in the standard success envelope. Human output also
prints the secret and unconfirmed status. Wekan does not await token insertion;
the response does not confirm that a session was stored. There is no returned
expiry, token listing, token import, or automatic credential storage. The request uses the selected
profile's existing credential; the target must be a user ID, not a username.

`user token delete <USER_ID>` requires exactly one of `--token-stdin` or `--all`.
Single-token input is UTF-8 through EOF, with one final LF or CRLF removed;
other whitespace is preserved. Empty input fails before HTTP. There is no
token argument or environment-variable input. Both deletion modes require
confirmation or `--yes`; JSON and piped invocations require `--yes`.

Deletion sends one JSON `POST /api/deletetoken`: `{userId, token}` for a supplied
token or `{userId}` for all tokens. Results contain `user_id`, `scope` (`single`
or `all`), `message`, `deletion_requested: true`, and `mutation_confirmed: false`.
The server's message can contain the target secret; the CLI redacts both that
secret and its authentication credential in the message and in error details.
Debug output never reveals tokens. Human rendering escapes terminal controls.

Wekan v11.06 has two verified limitations:

- **Single-token deletion acknowledges raw tokens without revoking them.** Its
  Accounts implementation compares the supplied string with stored `hashedToken`
  or legacy `token` fields without hashing it. A supplied stored hash matches;
  the CLI forwards input unchanged and never computes or substitutes a hash.
  Nonexistent tokens and users also receive unconfirmed acknowledgements.
- **Creation for a nonexistent user returns an unusable token.** The response
  does not prove that the user exists or that a session was persisted.

Delete-all revokes every existing session for the selected user, including the
caller's own session when targeting itself. Live readback confirms the saved
token field is removed; ordinary password login can create a new session afterward.
These commands retain profiles and local vault entries. To remove a credential
that was revoked remotely, run `wekan auth logout --local-only --yes`, then log in.
Self-targeted delete-all holds the selected credential's mutation lock through
confirmation and HTTP. A replaced, removed, or expired credential stops the
request before HTTP; concurrent local logout/login waits until the request ends.

Examples (PowerShell):

```powershell
# This success output contains the issued secret.
$issued = wekan --output json user token create $userId | ConvertFrom-Json
$issued.data.auth_token

# Native single-token request: a raw issued token does not match in v11.06.
$issued.data.auth_token | wekan user token delete $userId --token-stdin --yes

# Revoke all existing sessions for the target user.
wekan user token delete $userId --all --yes
```

## Output and errors

Lists use compact human tables. Detail and mutation commands use labelled
fields. Every string received from Wekan is escaped before terminal rendering.
JSON uses the standard `{ "ok": true, "data": ... }` envelope.

User detail output includes only the exact fields listed for `current` above.
This includes profile preferences, `createdThroughApi`, and `sessionData.totalHits`,
using stable snake_case output names. Workspace nodes map `id`, `name`, `icon`,
and recursive `children`; additional fields on a node or child produce
`protocol_error`. Services and password or token material are excluded.
Unmapped response fields are rejected rather than omitted.
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
