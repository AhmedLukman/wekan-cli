# Authentication

The implemented authentication surface supports registration, login, logout,
and live authentication status:

```text
wekan [--server <URL> | --profile <NAME>] [--output human|json] [--allow-insecure-http]
      auth register (--username <NAME> | --email <EMAIL> | both)
      [--password-stdin]

wekan [--server <URL> | --profile <NAME>] [--output human|json] [--allow-insecure-http]
      auth login (--username <NAME> | --email <EMAIL>)
      [--password-stdin]
      [--code | --code-stdin]

wekan [--server <URL> | --profile <NAME>] [--output human|json] [--allow-insecure-http]
      auth status

wekan [--server <URL> | --profile <NAME>] [--output human|json] [--allow-insecure-http]
      auth logout [--all | --local-only]
```

## Server selection

Authentication resolves its target through an explicit `--server` or
`--profile`, then `WEKAN_URL`, then `WEKAN_PROFILE`, and finally the persisted
active profile. Supplying both explicit selectors is a usage error. If no
selector resolves, the command returns `configuration_error` without reading
secrets or contacting Wekan. See [Server profiles and configuration](config.md)
for profile-management and storage behavior.

The URL must:

- use `https` or `http`;
- include a host;
- contain no embedded username, password, query, or fragment; and
- identify the Wekan base path, including any deployment subpath.

Target resolution validates and normalizes the server identity with a trailing
slash without granting network permission. Authentication endpoints are
resolved relative to that canonical identity. For example,
`https://example.test/wekan` becomes
`https://example.test/wekan/users/login` for login and
`https://example.test/wekan/api/user` for status. Remote logout uses
`https://example.test/wekan/users/logout`. Local-only logout still requires a
resolved target because it removes that target's native-vault entry.

Only client creation enforces network transport policy. Network authentication
requires HTTPS except for exact `localhost` and IPv4 or IPv6 loopback addresses;
`--allow-insecure-http` explicitly permits HTTP elsewhere for that invocation.
It does not disable certificate verification for HTTPS. Local-only logout uses
the canonical server identity directly and can remove a remote-HTTP target's
credential without the flag because it never creates a client or makes a
network request.

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

`auth status` reads the credential stored for the resolved target. It
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

## Logout

`auth logout` loads the credential for the resolved target and sends one
bearer-authenticated `POST users/logout` request. The default JSON body is
`{"all": false}` and revokes only the presented token. `--all` sends
`{"all": true}` and revokes every login token for that user, including browser
and other CLI sessions. The two modes do not prompt for confirmation.

Logout deliberately submits a locally expired credential because Wekan may
still have that token stored and be able to remove it. A missing record returns
`credential_not_found` without HTTP. HTTP 401 returns
`authentication_rejected`; redirects and other remote failures use the shared
transport/server error families.

Remote logout is strict and remote-first. The local credential is deleted only
after a complete HTTP 200 response containing the documented string `message`.
Any remote error preserves it for retry or diagnosis. Transport errors,
redirects, and 5xx errors set `outcome_unknown: true` and
`remote_logout_completed: null`: a sent request can be processed before an
intermediary returns one of those failures. An unusable HTTP 200 returns
`protocol_error` with the same indeterminate outcome and preserves the local
record. HTTP 200 is recorded as the observed status, but it is not proof that
logout completed until the body validates. If a validated remote success is
followed by a vault deletion failure, the command returns
`credential_store_failed` with `remote_logout_completed: true`.

After validated remote success, deletion is conditional on the stored record
still matching the record submitted to Wekan. If the record has been replaced
before that comparison, the newer credential is preserved and success reports
`credential_stored: true` and `local_credential_removed: false`. A deletion by
this invocation reports `local_credential_removed: true`; an already-absent
record reports `false`. Native-vault mutations use stable lock files in the
user's private local application-data directory, never the shared temporary
directory. Each transaction holds its target-account lock and a canonical-server
lock from before remote authentication work through its local save or deletion.
The whole transaction is therefore serialized across current CLI processes and
all direct or named aliases of a server: an all-token logout cannot retain a
token that a concurrent login had already created, and a successful logout
cannot remove a concurrently saved login.

An externally changed record can still be preserved by the conditional delete.
For an all-token logout, human output tells the caller to verify that record
before using it because an older CLI or another vault writer might have stored
a token that the remote all-token operation revoked.

`--local-only` conflicts with `--all`, makes no HTTP request, and deletes the
resolved target's vault entry without loading or decoding it. It therefore
clears expired, rejected, malformed, or unsupported records. An absent entry is
an idempotent success with `local_credential_removed: false`. Human output
explicitly states that no Wekan tokens were revoked; any still-valid remote
token remains usable until revoked or expired.

## Request safety and server errors

Registration, login, and remote logout send one JSON POST request; status sends
one GET request. All remote operations use a 10-second connect timeout, a
30-second total timeout, and a 1 MiB response limit. Redirects are not followed
and automatic retries are disabled. In particular, logout mutation requests
are never automatically retried. Successful registration and login return an
authentication session containing `id`, `token`, and RFC 3339 `tokenExpires`.

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

No plaintext fallback exists. The entry uses service `wekan-cli`. Direct URL
selection uses the canonical server URL as its account key; named selection
uses `profile:<store-identity>:<name>`, where the opaque store identity is
derived from the canonical profile configuration directory. A later successful
registration or login replaces only that target's entry. Replacing the local
record does not revoke older tokens on Wekan.

Two named profiles can therefore maintain independent credentials even when
they use the same canonical URL, and identically named profiles in separate
`WEKAN_CONFIG_DIR` stores cannot collide. Named profiles never copy or fall
back to an existing URL-keyed credential; those direct credentials remain
usable only through direct URL selection. Named authentication operations hold
a shared profile-store lease through target resolution, network work, and vault
mutation so a profile update or removal cannot race the transaction.

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

Registration, login, logout, and status success data includes a nullable
`profile` field. It contains the selected profile name for a named target and
is `null` for direct URL selection. Named-target errors include the same profile
context.

Credential reads distinguish an absent entry from an unavailable vault or an
invalid record. Missing credentials are an unauthenticated state; vault access,
decoding, version, server-key, and field-validation failures use the credential
error family. Status never repairs, replaces, or removes a record. Normal
logout reads and validates before remote revocation; local-only logout bypasses
record decoding and directly deletes the vault entry.

A credential-store write can fail after Wekan reports success. Registration
then returns `credential_store_failed` with `account_created: true`; login uses
the same error code with `session_created: true`. The CLI discards the in-memory
token and does not attempt remote rollback or revocation.

A credential-store deletion can likewise fail after Wekan completes logout.
That failure reports `credential_store_failed` with
`remote_logout_completed: true`; the CLI does not retry the remote mutation.
