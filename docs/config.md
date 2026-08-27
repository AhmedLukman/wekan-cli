# Server profiles and configuration

Named profiles associate a portable local name with a canonical Wekan server
URL. They contain no credentials, and profile-management commands never contact
Wekan.

```text
wekan profile add <NAME> <URL> [--use]
wekan profile list
wekan profile show <NAME>
wekan profile use <NAME>
wekan profile update <NAME> <URL>
wekan profile remove <NAME> [--force] [--yes]
```

Profile names must match `[a-z0-9][a-z0-9._-]{0,63}`. Names are immutable and
unique. Multiple profiles can point to the same canonical server URL; their
credentials remain independent.

The first added profile becomes active. Later additions become active only with
`--use`. `profile use` is idempotent, as are `profile show` and an update to the
profile's existing URL. Updating a profile changes only its URL.

Removing the active profile requires `--force`. A forced removal clears the
active selection and does not choose a replacement. Updating or removing a
profile that has a stored credential is refused, including when `--force` is
used. First clear only that profile's credential:

```text
wekan --profile <NAME> auth logout
wekan --profile <NAME> auth logout --local-only
```

The first form revokes the token remotely before deleting it; the second only
deletes the local credential.

Profile removal also requires confirmation. In interactive human mode the
prompt identifies the profile and server, and states when the active selection
will be cleared. Answering no exits successfully without changing the profile
store. JSON or non-terminal invocations must pass command-local `--yes`.
`--force` and `--yes` are independent: removing an active profile requires
both. The CLI releases its preview lease while prompting, then reacquires the
exclusive mutation lease and refuses to remove a profile whose server, active
state, existence, or credential state changed after confirmation.

## Target selection

Ordinary Wekan commands resolve a profile name and an optional supplied URL
independently. The profile name is selected by `--profile`, then
`WEKAN_PROFILE`, then the persisted active profile, and finally the literal
name `default`. The supplied URL is selected by `--server`, then `WEKAN_URL`.
`--profile` and `--server` may be used together.

An existing profile supplies its stored URL when no URL is supplied. A supplied
URL is canonicalized and must match the stored URL exactly; otherwise the
command returns `profile_server_mismatch` without changing the profile or
accessing its credential.

A missing profile returns `profile_not_found` for status, remote logout, and
login or registration without a supplied URL. `auth logout --local-only` can
instead remove that missing profile's orphaned vault entry when a URL is
supplied with `--server` or `WEKAN_URL`; it does not recreate profile metadata.
Login and registration with a supplied URL may initialize the missing profile,
but only after Wekan returns a complete, valid authentication session. A profile
created this way becomes active only when the profile store is empty. Later
profiles remain inactive; selecting one for authentication does not activate it.

Profile-management commands reject explicit `--server` and `--profile`
selectors and ignore `WEKAN_URL` and `WEKAN_PROFILE`.

Server URLs are canonicalized with a trailing slash and must use HTTP or HTTPS,
include a host, and contain no username, password, query, or fragment. This
canonical identity validation is independent of permission to contact the
server. A remote HTTP URL can therefore be added, updated, selected, inspected,
or used for local credential deletion without `--allow-insecure-http`.

When a handler actually creates an HTTP client, HTTPS is required except for
exact localhost and IP loopback addresses. A network command targeting remote
HTTP must pass `--allow-insecure-http` on that invocation. Persisting the URL in
a profile never grants future network permission.

## Profile storage

The profile document is `profiles.json` beneath the platform configuration
directory returned for the `wekan-cli` application. Set `WEKAN_CONFIG_DIR` to
an absolute directory to override that location, for example in isolated test
or automation environments. Relative overrides are rejected.

The strict version-1 schema is:

```json
{
  "version": 1,
  "revision": 1,
  "active_profile": "work",
  "profiles": {
    "personal": { "server": "https://personal.example/" },
    "work": { "server": "https://work.example/wekan/" }
  }
}
```

`active_profile` can be `null`. Entries are stored and rendered in name order.
Unknown fields, unsupported versions, invalid names or URLs, inconsistent
active selections, malformed JSON, and documents larger than 1 MiB produce
`configuration_error`.

Reads take a cross-process shared lease. Mutations take an exclusive lease and
replace the file through a flushed same-directory temporary file, retaining a
backup until replacement succeeds so an interrupted replacement is
recoverable. On Unix, the profile directory and files created by the CLI use
private permissions. A pre-existing `WEKAN_CONFIG_DIR` retains its existing
directory permissions. Unreadable or unwritable storage is reported as
`configuration_error`. Write errors distinguish failure before the replacement
was installed from a failure to finalize an installed replacement, so
structured profile state does not incorrectly claim that a visible profile was
never created.

## Credential scope

Profiles never store secrets. The native credential vault uses service
`wekan-cli` and the account key `profile:<store-identity>:<name>`, where the
opaque store identity is derived from the canonical profile configuration
directory. The version-1 credential record includes and validates the expected
canonical server URL. Two profiles for the same server, and identically named
profiles in separate `WEKAN_CONFIG_DIR` stores, therefore have independent
vault accounts. No migration or compatibility fallback is attempted for
credentials written by older builds.

Existing-profile authentication holds a shared profile-store lease from target
resolution through the complete vault/network transaction. Initializing a
missing profile holds an exclusive mutation lease through authentication,
profile persistence, and credential creation. All paths acquire the profile
lease before credential mutation guards, preventing profile changes and
cross-process initialization races between Wekan CLI processes.

## Output and errors

Profile objects have `name`, `server`, and `active` fields. JSON list output is
`{"active_profile": string|null, "profiles": [...]}`. Removal reports the
removed name and server, `removed: true`, and the resulting nullable active
profile. Human list output has deterministic Name, Server, and Active columns;
an empty store prints a clear no-profiles message.

Missing, duplicate, mismatched-server, active-in-use, and credential-bearing
profiles use the stable codes `profile_not_found`, `profile_already_exists`,
`profile_server_mismatch`, `profile_in_use`, and `profile_has_credential`, all
with exit status 3. `credential_already_exists` also uses exit 3. Native-vault
failures retain the credential error family and exit status 6.
