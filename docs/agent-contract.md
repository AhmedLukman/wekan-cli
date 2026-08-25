# Agent output contract

Use `--output json` for deterministic machine-readable output. Every envelope
is one JSON object followed by a newline.

## Streams

- Successful command envelopes are written to stdout and stderr is empty.
- Error envelopes are written to stderr and stdout is empty.
- `--help` and `--version` remain human-readable text on stdout.
- Human-mode errors are written to stderr.

## Authentication success

Registration and login use the same secret-free data shape:

```json
{
  "ok": true,
  "data": {
    "server": "https://wekan.example/",
    "user_id": "XQMZgynx9M79qTtQc",
    "token_expires": "2030-01-02T03:04:05Z",
    "credential_stored": true
  }
}
```

The `server` value is canonicalized. The returned token is absent because it is
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

## Errors

```json
{
  "ok": false,
  "error": {
    "code": "login_rate_limited",
    "message": "too many failed login attempts; try again later; retry after 30 seconds",
    "details": {
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
- `retry_after_seconds`: a valid integer `Retry-After` value from HTTP 429; and
- `token_expires`: the stored RFC 3339 expiry when a credential is locally
  expired.

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

Status uses `credential_not_found` when the canonical server has no saved
record, `credential_expired` when the saved expiry is not in the future, and
`authentication_rejected` when Wekan rejects the bearer token. Wekan v11.06's
current-user route serializes rejection inside HTTP 200, so details contain
both the actual `http_status` and the embedded `wekan_status_code`. Status is
read-only and never removes the offending record.

Messages are intended for humans. Automation must branch on `error.code` and
the process exit status. Stable error codes are:

- `invalid_input`
- `configuration_error`
- `insecure_transport`
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

## Exit statuses

| Exit | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Unexpected internal failure |
| `2` | CLI usage, missing/ambiguous identity, empty secret, or registration password mismatch |
| `3` | Missing or invalid server configuration, including insecure transport refusal |
| `4` | Transport, redirect, or protocol failure, including an unreadable, malformed, or oversized HTTP 200 response |
| `5` | Unauthenticated state, authentication rejection, or another Wekan application/server rejection |
| `6` | Credential store unavailable or failed, including a created account or session whose token could not be stored |

When an error response body cannot be read safely, its HTTP status still
determines the error classification and exit status.
