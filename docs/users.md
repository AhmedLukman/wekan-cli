# Users

The `wekan user` family covers the complete Wekan v11.06 `/api/user` and
`/api/users` REST slice: eight operations across five paths, with one bounded
client method per operation. Every command requires an existing profile and a
stored credential whose expiry is still in the future.

```text
wekan user current
wekan user cards [--due] [--from <RFC3339>] [--to <RFC3339>]
wekan user list
wekan user get <USER>
wekan user create --username <NAME> --email <EMAIL> [--password-stdin]
wekan user boards <USER_ID>
wekan user take-ownership <USER_ID> [--yes]
wekan user disable-login <USER_ID> [--yes]
wekan user enable-login <USER_ID>
wekan user delete <USER_ID> [--yes]
```

`current` calls `GET /api/user`. Its exact allowlist is `_id`, `username`,
emails (`address`, `verified`), `profile.fullname`, `isAdmin`, `loginDisabled`,
`authenticationMethod`, `createdAt`, `modifiedAt`, `lastConnectionDate`, typed
organization/team memberships, and typed board-role flags. The shared client
operation also powers
`auth status`, but the commands intentionally project it differently:
`user current` is resource-focused, while `auth status` adds local profile,
credential-presence, and token-expiry metadata.

`cards` calls `GET /api/user/cards`. Wekan returns non-archived cards on active
boards readable by the caller where the caller is a card member, assignee,
requester, or assigner. `--due` requires a due date. `--from` and `--to` accept
RFC 3339 timestamps; either bound enables due-date filtering, and the CLI
rejects a range in which `from` is later than `to` before HTTP. Missing or
invalid authentication is an HTTP 401 response on this route, unlike the
HTTP-200 embedded authentication error returned by `GET /api/user`.

`list`, `get`, `create`, `take-ownership`, `disable-login`, `enable-login`, and
`delete` require a Wekan site administrator. `get` accepts a user ID or
username. `boards` requires either the selected user's own credential or a site
administrator and returns active, non-archived, non-internal board ID/title
summaries. The authenticated-self form is also exposed as `wekan board list`.
Wekan usernames are optional, so `list` emits JSON `null` and the
human table displays `<none>` for an email-only account.

## Creating users

The new password is never accepted as a command-line value or environment
variable. Interactive use reads and confirms it without echo. Automation uses
`--password-stdin` and supplies exactly one password line. The password and
stored bearer token are redacted from every failure and never appear in output.

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
the affected board summaries. Targeting oneself is rejected locally.

`disable-login` sends `disableLogin`, sets the target's `loginDisabled` field,
and clears their existing Wekan sessions. Targeting oneself is rejected
locally. Wekan v11.06's REST login handler does not inspect `loginDisabled`, so
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

## Output and errors

Lists use compact human tables. Detail and mutation commands use labelled
fields. Every string received from Wekan is escaped before terminal rendering.
JSON uses the standard `{ "ok": true, "data": ... }` envelope.

User detail output includes only the exact fields listed for `current` above.
All other profile fields, `createdThroughApi`, `services`, `sessionData`,
password or token material, UI preferences, and unknown Wekan fields are
excluded. Verified HTTP or embedded status 403 maps to `permission_denied`.
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
