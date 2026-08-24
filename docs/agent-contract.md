# Agent output contract

Use `--output json` for deterministic machine-readable output. Every envelope
is a single JSON object followed by a newline.

## Streams

- Successful command envelopes are written to stdout and stderr is empty.
- Error envelopes are written to stderr and stdout is empty.
- `--help` and `--version` remain human-readable text on stdout.
- Human-mode errors are written to stderr.

## Registration success

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

The `server` value is canonicalized. The returned login token is deliberately
absent because it is stored in the native credential store.

## Errors

```json
{
  "ok": false,
  "error": {
    "code": "registration_disabled",
    "message": "registration is disabled on the Wekan server",
    "details": {
      "http_status": 403
    }
  }
}
```

`details` is always an object. Depending on the error it can contain:

- `http_status`: HTTP status returned by the server;
- `server_error` and `server_reason`: structured Wekan error fields;
- `account_created`: whether account creation is known to have occurred; and
- `outcome_unknown`: whether the server may have processed a failed request.

Registration HTTP 400 errors always set `outcome_unknown: true` because Wekan
`v11.06` can return that status both before and after creating the account. HTTP
5xx responses also set it because an intermediary can fail after forwarding a
non-idempotent registration request.

Messages are intended for humans. Automation must branch on `error.code` and
the process exit status.

Stable error codes are:

- `invalid_input`
- `configuration_error`
- `insecure_transport`
- `transport_error`
- `unexpected_redirect`
- `protocol_error`
- `registration_rejected`
- `registration_disabled`
- `server_error`
- `credential_store_unavailable`
- `credential_store_failed`
- `internal_error`

Passwords and tokens are redacted from all fields.

## Exit statuses

| Exit | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Unexpected internal failure |
| `2` | CLI usage, missing identity, empty password, or password mismatch |
| `3` | Missing or invalid server configuration, including insecure transport refusal |
| `4` | Transport, redirect, or protocol failure, including an unreadable, malformed, or oversized HTTP 200 response |
| `5` | Wekan returned a non-success status, including when its response body is unreadable or oversized |
| `6` | Credential store unavailable or failed, including partial registration |

When an error response body cannot be read safely, its HTTP status still
determines the error classification and exit status.
