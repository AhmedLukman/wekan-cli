# Authentication

The implemented authentication surface supports registration, login, and live
authentication status:

```text
wekan [--server <URL>] [--output human|json] [--allow-insecure-http]
      auth register (--username <NAME> | --email <EMAIL> | both)
      [--password-stdin]

wekan [--server <URL>] [--output human|json] [--allow-insecure-http]
      auth login (--username <NAME> | --email <EMAIL>)
      [--password-stdin]
      [--code | --code-stdin]

wekan [--server <URL>] [--output human|json] [--allow-insecure-http]
      auth status
```

Logout, named profiles, configuration files, and multi-account selection are
not implemented in this milestone.

## Server selection

`--server` takes precedence over `WEKAN_URL`. If neither is set, the command
returns `configuration_error` without reading secrets or contacting Wekan.

The URL must:

- use `https` or `http`;
- include a host;
- contain no embedded username, password, query, or fragment; and
- identify the Wekan base path, including any deployment subpath.

The CLI normalizes the base URL with a trailing slash and resolves the
authentication endpoint relative to it. For example,
`https://example.test/wekan` becomes
`https://example.test/wekan/users/login` for login and
`https://example.test/wekan/api/user` for status.

HTTPS is required except for exact `localhost` and IPv4 or IPv6 loopback
addresses. `--allow-insecure-http` explicitly permits HTTP elsewhere. It does
not disable certificate verification for HTTPS.

Loopback HTTP bypasses configured system and environment proxies so plaintext
authentication secrets cannot leave the local machine through a proxy. HTTPS
and explicitly permitted remote HTTP use the system proxy policy.

## Identity and secret input

Registration requires at least one nonempty `--username` or `--email`, and
allows both. Interactive registration reads a new password twice without echo.

Login requires exactly one nonempty `--username` or `--email`. Wekan remains
responsible for credential validity and email syntax. Interactive login reads
the existing password once without echo.

For a two-factor account, pass `--code` to prompt for a code without echo. This
mode uses interactive password input and sends the password and code together
in one request. The CLI never submits a password-only request and then replays
it automatically.

Automation passes `--password-stdin`. For two-factor login it must also pass
`--code-stdin`, with the password on the first line and the code on the second:

```text
password
123456
```

Each input removes only its `LF` or `CRLF` ending and preserves all other
whitespace. An empty line or end-of-file is an input error. `--code-stdin`
requires `--password-stdin`; interactive `--code` conflicts with both stdin
flags. Password and code value arguments, `WEKAN_PASSWORD`, and code environment
variables are deliberately unsupported.

## Authentication status

`auth status` reads the credential stored under the canonical server URL. It
does not accept command-specific arguments, prompt for secrets, or mutate the
credential store.

The stored version-1 record must contain the same canonical server URL, a
nonempty user ID and token, and an RFC 3339 expiry. A missing record returns
`credential_not_found`. A record whose expiry is at or before the current time
returns `credential_expired` with `token_expires` details and is not sent to
Wekan.

For a nonexpired record, the CLI sends one bearer-authenticated `GET api/user`
request. Wekan v11.06 reports a missing or invalid token as HTTP 200 with an
embedded `statusCode: 401`; the CLI maps this to `authentication_rejected` and
reports both `http_status: 200` and `wekan_status_code: 401`. It does not delete
the rejected credential.

Successful status requires the returned `_id` to be nonempty and to match the
stored user ID. Output allowlists only the user ID, username, profile full name,
administrator flag, email addresses and verification flags. Board memberships,
other profile data, and all service/session data are discarded.

## Request safety and server errors

Registration and login send one JSON POST request; status sends one GET request.
All use a 10-second connect timeout, a 30-second total timeout, and a 1 MiB
response limit. Redirects are not followed and automatic retries are disabled.
Successful registration and login return an authentication session containing
`id`, `token`, and RFC 3339 `tokenExpires`.

Registration accepts only HTTP 200 as success. HTTP 400 maps to
`registration_rejected`; HTTP 403 maps to `registration_disabled`; other
non-success responses map to `server_error`.

Login accepts only HTTP 200 as success. A body-parser failure can return HTTP
400, which maps to `protocol_error`. Wekan returns HTTP 401 for invalid request
shape, credentials, or two-factor code; the CLI maps it to `login_rejected`.
HTTP 429 maps to `login_rate_limited`. A valid integer `Retry-After` response
header is exposed as `retry_after_seconds`. Other non-success responses map to
`server_error`.

When HTTP 401 contains Wekan's `no-2fa-code` error, the CLI still returns
`login_rejected` but sets `two_factor_required: true` and tells the caller to
retry with `--code` or `--code-stdin`. It does not automatically replay login.

Wekan `v11.06` can return HTTP 400 after creating a user if login-token
insertion then fails, so registration 400 errors set `outcome_unknown: true`.
A transport failure or 5xx response for either mutating authentication request
can also have an unknown remote outcome and is never retried automatically.

An unusable HTTP 200 registration response sets `account_created: true`; an
unusable HTTP 200 login response sets `session_created: true`. These fields
record that Wekan reported success even though the CLI could not safely use its
response.

## Credential storage

Before reading any secret or contacting Wekan, the CLI checks the native
credential store:

- Windows Credential Manager on Windows;
- Keychain Services on macOS; and
- Secret Service on Linux and other supported Unix systems.

No plaintext fallback exists. The entry uses service `wekan-cli` and the
canonical server URL as its account key. One active local credential is kept per
server; a later successful registration or login replaces that entry. Replacing
the local record does not revoke older tokens on Wekan.

The stored secret is a versioned JSON record:

```json
{
  "version": 1,
  "server_url": "https://wekan.example/",
  "user_id": "XQMZgynx9M79qTtQc",
  "token": "stored-only-in-the-native-vault",
  "token_expires": "2030-01-02T03:04:05Z"
}
```

Passwords and two-factor codes are never stored. Tokens are never rendered in
human or JSON output.

Credential reads distinguish an absent entry from an unavailable vault or an
invalid record. Missing credentials are an unauthenticated state; vault access,
decoding, version, server-key, and field-validation failures use the credential
error family. Status never repairs, replaces, or removes a record.

A credential-store write can fail after Wekan reports success. Registration
then returns `credential_store_failed` with `account_created: true`; login uses
the same error code with `session_created: true`. The CLI discards the in-memory
token and does not attempt remote rollback or revocation.
