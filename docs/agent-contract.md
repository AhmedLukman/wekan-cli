# Agent output contract

Use `--output json` for deterministic machine-readable output. Every envelope
is one JSON object followed by a newline.

## Streams

- Successful command envelopes are written to stdout and stderr is empty.
- Error envelopes are written to stderr and stdout is empty.
- `--help` and `--version` remain human-readable text on stdout.
- Human-mode errors are written to stderr.
- Interactive confirmation prompts are written to stderr. JSON output never
  prompts; destructive automation must pass command-local `--yes`.

## Destructive confirmation

`auth logout` in every scope and `profile remove` require interactive
confirmation or `--yes`. Normal target and removability preflight runs first;
its errors retain their documented codes. Otherwise, non-terminal and JSON
invocations without `--yes` return `invalid_input` with exit status 2 before
destructive work. `--yes` is accepted only by those commands. For
active-profile removal, `--force` remains a separate requirement.

Logout holds the selected credential target's mutation guards while awaiting
interactive confirmation, binding approval to that target state and preventing
a concurrent login or logout from replacing it before execution.

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

`operation` is `auth_logout` or `profile_remove`. Human output is
`Cancelled; no changes made.`

## Authentication success

Registration and login use the same secret-free data shape:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": null,
    "user_id": "XQMZgynx9M79qTtQc",
    "token_expires": "2030-01-02T03:04:05Z",
    "credential_stored": true
  }
}
```

The `server` value is canonicalized. `profile` is the selected named profile or
`null` for a direct URL target. The returned token is absent because it is
stored in the native credential store. Passwords and two-factor codes are also
never rendered.

Authentication status uses a nested allowlisted profile. Optional scalar
profile fields are always present and use `null` when Wekan omits them; missing
emails use an empty array, and optional email-entry fields also use `null`.

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": null,
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
    "profile": null,
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
from an idempotent already-absent result. If remote logout succeeds but the
record was concurrently replaced, the newer record is preserved and success
sets `credential_stored: true` and `local_credential_removed: false`.

For example, a local-only deletion is:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "profile": null,
    "logout_scope": "local_only",
    "remote_logout_completed": false,
    "credential_stored": false,
    "local_credential_removed": true
  }
}
```

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
      "profile": null,
      "http_status": 429,
      "server_error": "too-many-requests",
      "retry_after_seconds": 30
    }
  }
}
```

`details` is always an object. Depending on the error it can contain:

- `http_status`: HTTP status returned by the server;
- `wekan_status_code`: application status serialized inside a Wekan response;
- `server_error` and `server_reason`: structured Wekan error fields;
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
- `profile`: the named target associated with an authentication error, or
  `null` for a direct target, when target context is relevant.

Registration HTTP 400 errors always set `outcome_unknown: true` because Wekan
`v11.06` can return that status before or after account creation. Authentication
transport failures and HTTP 5xx responses also set it because an intermediary
can fail after forwarding the request.

An invalid, unreadable, or oversized HTTP 200 registration response sets
`account_created: true`; the login equivalent sets `session_created: true`.
A vault write failure uses the same operation-specific field because it happens
after a validated success response.

Login HTTP 400 is a malformed-request `protocol_error`. HTTP 401 is
`login_rejected`; when Wekan specifically returns `no-2fa-code`, details include
`two_factor_required: true` so automation can request a code without depending
on the upstream error string. The CLI never retries that request automatically.

Status uses `credential_not_found` when the resolved target has no saved record,
`credential_expired` when the saved expiry is not in the future, and
`authentication_rejected` when Wekan rejects the bearer token. Wekan v11.06's
current-user route serializes rejection inside HTTP 200, so details contain
both the actual `http_status` and the embedded `wekan_status_code`. Status is
read-only and never removes the offending record.

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
  "profile": null,
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
- `profile_in_use`
- `profile_has_credential`
- `transport_error`
- `unexpected_redirect`
- `protocol_error`
- `login_rejected`
- `login_rate_limited`
- `credential_not_found`
- `credential_expired`
- `authentication_rejected`
- `registration_rejected`
- `registration_disabled`
- `server_error`
- `credential_store_unavailable`
- `credential_store_failed`
- `internal_error`

Passwords, two-factor codes, and tokens are redacted from all fields.

The four `profile_*` codes use exit status 3. Malformed, unsupported,
unreadable, or unwritable profile storage uses `configuration_error`; native
vault failures retain the credential error family and exit status 6.

## Exit statuses

| Exit | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Unexpected internal failure |
| `2` | CLI usage, missing/ambiguous identity, empty secret, or registration password mismatch |
| `3` | Missing or invalid server/profile configuration, profile conflicts, or insecure transport refusal |
| `4` | Transport, redirect, or protocol failure, including an unreadable, malformed, or oversized HTTP 200 response |
| `5` | Unauthenticated state, authentication rejection, or another Wekan application/server rejection |
| `6` | Credential store unavailable or failed, including a created account or session whose token could not be stored |

When an error response body cannot be read safely, its HTTP status still
determines the error classification and exit status.
