# CLI updates

## Commands

```console
wekan self update [--check | --yes] [--output human|json]
```

`--check` reports the latest stable release without installing. Without either
flag, an available update requires terminal confirmation, defaulting to No.
`--yes` authorizes installation without a prompt and conflicts with `--check`.
No confirmation is needed when the installed version is already current.

These local commands do not read Wekan profiles or credentials. Explicit
`--server` and `--profile` are rejected; `WEKAN_URL` and `WEKAN_PROFILE` are
ignored. The inherited global `--allow-insecure-http` does not apply to release
downloads, which always require HTTPS. Raw output is unsupported.

Manual checks always contact GitHub, bypassing the notification cache and
`WEKAN_NO_UPDATE_NOTIFIER`. They use a 15-second request timeout. Only stable,
published GitHub releases are considered; semantic version precedence prevents
downgrades, and build metadata alone does not establish a newer version. A newer
major version is eligible with the same explicit installation confirmation.

## Interactive startup

After argument validation, eligible terminal commands check before resolving
Wekan configuration or executing the requested command. Eligibility requires
human output and terminal stdin, stdout, and stderr, with no CI environment or
`WEKAN_NO_UPDATE_NOTIFIER` variable. Setting that variable to **any value**,
including an empty string, disables automatic checks. Help, version, usage
errors, and `self update` commands do not invoke the automatic checker.

Check attempts, release metadata, and notices are cached for 24 hours beneath
the platform application cache directory, in `updates/check.json`. This cache is
independent of `WEKAN_CONFIG_DIR` and Wekan profiles. Automatic network requests
time out after two seconds; connectivity, rate-limit, and cache failures quietly
continue the requested command. Concurrent starts share a nonblocking cache lock.

An available update prompts on stderr:

```text
A new Wekan CLI version is available: 2.0.0 → 2.1.0
Update now? [y/N]
```

Declining continues the original command and suppresses another notice for
24 hours. An unrelated command's `--yes` does not answer this prompt. Successful
installation exits with `Updated successfully; rerun your command.` The original
command is not executed after accepting installation, including when installation
fails; failures produce an explicit error and nonzero exit status.

## Installation methods

The updater matches metadata to the running executable and its compiled version.
It never guesses ownership from the presence of Cargo or the executable's name
alone. Conflicting matching records are treated as unrecognized.

- Official shell/PowerShell installations: match the cargo-dist receipt, fetch
  the archive for the current platform and exact offered release, verify its
  published SHA-256 checksum, replace only the executable, preserve permissions,
  and refresh the receipt's version while retaining other metadata.
- Tracked crates.io installations: invoke `cargo install wekan-cli --locked`
  with the exact offered version, original root, target, profile, and feature
  selection. The version must also be available on crates.io and supported by
  the local Rust toolchain. Cargo output is captured, including in JSON mode.
- Manually copied archives, source/git installs, missing/corrupt metadata, and
  ambiguous records: automatic checks show the release link and manual guidance.
  `--check` remains available, while installation returns `update_failed`.

Installations are locked during updates. Windows replacement accounts for the
running executable's file lock. Cargo updates keep the current version runnable
during download and compilation, so interrupting that work leaves the installed
CLI available. A receipt-write failure after replacement reports that the binary
was updated but its metadata needs repair with the original installer.

## Structured output

`wekan self update --check --output json` produces the normal success envelope:

```json
{"ok":true,"data":{"current_version":"2.0.0","latest_version":"2.1.0","status":"available","installation_method":"release","release_url":"https://github.com/AhmedLukman/wekan-cli/releases/tag/v2.1.0"}}
```

All five fields are always present. `current_version` identifies the version
running when the invocation started, including after replacement. `latest_version`
is the discovered stable release. `status` is `up_to_date`, `available`, or
`updated`; for `updated`, the installed version is `latest_version`.
`installation_method` is `release`, `cargo`, or `unknown`.

JSON or non-terminal installation without `--yes` returns `invalid_input`, exit 2,
when confirmation would be required. Declining an interactive manual update
returns the existing cancellation result with `operation: "self_update"` and
exit 0. Installation failures use `update_failed`, exit 4; network, protocol, and
filesystem failures retain their existing classifications. Successful JSON
commands leave stderr empty; failures leave stdout empty.
