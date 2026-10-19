# Wekan CLI

Wekan CLI is an agent-first, human-friendly command-line client for Wekan. It
provides deterministic commands, stable JSON output, explicit errors, native
credential storage, and confirmation gates for destructive operations.

Version 1.0.0 targets **Wekan v11.06 exactly**. Other Wekan versions are not
currently supported or tested.

## Compatibility

Starting with v1.0.0, the documented command syntax, JSON field names and types,
error codes, exit statuses, and configuration behavior follow Semantic
Versioning. Incompatible changes require a new major version. Compatible
releases can add commands, optional flags, and JSON or error-detail fields;
automation should accept additional output fields and use documented error
codes and exit statuses instead of parsing human-readable messages.

Human output, help text, and error wording can change between releases. Raw
API bodies and native export contents follow the supported Wekan server's
behavior. The Rust library modules support the CLI implementation and are not
a supported Rust SDK. See the
[command contract](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/command-contract.md)
and [agent contract](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/agent-contract.md)
for the supported CLI interfaces.

## Install

Rust 1.88 or newer can build the published crate:

```console
cargo install wekan-cli --locked
```

GitHub Releases also provides checksummed archives for Windows x64, macOS Intel
and Apple Silicon, and Linux x64 and ARM64. The release installers select the
right archive and install `wekan` into Cargo's bin directory.

macOS and Linux:

```console
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/AhmedLukman/wekan-cli/releases/latest/download/wekan-cli-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/AhmedLukman/wekan-cli/releases/latest/download/wekan-cli-installer.ps1 | iex"
```

The GitHub binaries are not signed with paid Apple or Windows publisher
certificates, so Gatekeeper or SmartScreen may show an unknown-developer
warning. Release assets include SHA-256 checksums and GitHub build provenance.
With the GitHub CLI installed, an asset can be verified with:

```console
gh attestation verify PATH-TO-ASSET -R AhmedLukman/wekan-cli
```

## Quick start

Login can initialize the default profile after Wekan accepts the credentials:

```console
wekan --server https://wekan.example/ auth login --username alice
wekan auth status
wekan board list
```

Use JSON output for scripts and agents:

```console
wekan --output json board list
```

Scoped commands name their parent IDs explicitly:

```console
wekan card get CARD_ID --board BOARD_ID --list LIST_ID
wekan comment list --board BOARD_ID --card CARD_ID
```

Passwords and optional two-factor codes are read without echo. Automation can
use `--password-stdin` and `--code-stdin`; secrets are deliberately unsupported
as command-line values or environment variables. Login tokens are stored in
the operating system's native credential vault.

For a local or explicitly managed profile:

```console
wekan profile add local http://localhost:3000 --use
wekan --profile local auth login --username alice
```

## Agent skill

Install the portable discovery skill for your agent, separately from the CLI:

```console
npx skills add AhmedLukman/wekan-cli --skill wekan-cli --agent codex
```

For Claude Code, replace `--agent codex` with `--agent claude-code`. Add
`--global` for a user-wide install. GitHub CLI and manual copying also work
without Node.js; see [skill installation and commands](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/skills.md).

The small skill directs the agent to guidance embedded in the installed
executable:

```console
wekan skill show
wekan skill list
wekan skill show cards
```

These commands work offline without a server, profile, or credentials. Updating
the CLI updates its detailed guide. Use command-specific `--help` for commands,
arguments, and options, and read additional topics for behavior beyond help.

## Command coverage

The CLI includes profile and authentication management plus typed commands for
users, boards, lists, swimlanes, cards, comments, checklists, checklist items,
custom-field definitions, dropdown items, attachments, card dependencies,
board integrations, automation rules, and global administrator settings. It
implements all 144 audited REST operations in the corrected Wekan v11.06 contract,
including dedicated card archive/restore, custom-field assignment, custom-field
lookup, all 12 attachment transfer operations, all ten native exports, all five dependency operations, and all
seven board/card membership mutations, all seven integration operations,
all five board automation rule operations, bulk card creation/deletion/label changes,
and board label addition. Native board/card/list/swimlane copies, list/swimlane
moves, board/list card counts, swimlane card listings, and card lookup by ID
alone are also available; see [copy and move](docs/copy-and-move.md) and
[card reads](docs/cards.md). Nested `board member`, `card member`,
and `card assignee` commands support focused lists and individual changes;
board members also support named roles and individual permission flags.
`rule` commands manage board automation with typed JSON definitions, scheduled
and button payloads, and strict human/JSON output; see [rules](docs/rules.md).

`user token create <USER_ID>` issues a token and prints its secret in successful
human/JSON output. `user token delete <USER_ID> (--token-stdin | --all) [--yes]`
requests revocation without changing local credentials. Both require site
administration. Wekan v11.06 matches stored hashes rather than raw issued tokens
for single-token deletion. See
[user API tokens](docs/users.md#api-tokens) for native behavior and examples.

`admin settings get/update` and `admin attachment-settings get/update` configure
global REST settings and attachment storage through typed flags or JSON file/stdin
input, with credential files/stdin and masked presence output. See
[administrator settings](docs/admin-settings.md).

`admin domain list`, `admin org list`, and `admin team list` report domain user
counts and resource feature summaries. Organization/team `features update` and
confirmed `features update-all` commands complete all 11 administrator operations.
See [administrator commands](docs/admin.md).

`integration` commands manage board webhooks and their activity subscriptions,
including token input through stdin and typed mutation results;
see [integrations](docs/integrations.md).
`board card-settings get/update` exposes card display controls and aging thresholds;
`board domain list/add/remove` exposes native sharing endpoints. Domain listing
requires a site administrator; valid sharing writes preserve Wekan v11.06's
`not-authorized` failure. See [board settings and sharing](docs/boards.md#card-settings-and-aging).
Dependency commands support board
and card listings, create-or-update, nullable metadata updates, and confirmed
removal with typed output. Attachment transfers support files
and stdin/stdout; pinned Wekan
upload/copy/move failures and ignored list selectors remain explicitly documented.
`board export`, `card export`, and `attachment export` deliver native artifacts
to protected file destinations or validated raw stdout. Formats, selection controls,
authentication, and pinned server failures are described in [exports](docs/exports.md).
`board attachment list --board <ID>` returns native legacy attachment links,
including board backgrounds. `system schema-upgrade-status` reports public
startup migration progress, and `auth preflight <login|register|logout>` probes
the three anonymous OPTIONS endpoints. Public probes accept `--server` or
`WEKAN_URL` before a profile or login exists. See [system status](docs/system.md).
The same-origin `wekan api request` escape hatch remains available for raw requests.

`board import`, `board import-zip`, `board import-trello`, and `card import-ics`
cover all five native import operations, accepting files or stdin with source selection, member maps,
CSV/XLSX encoding, and explicit timeout controls. See [imports](docs/imports.md)
for inputs, results, limits, and pinned server defects.

Run `wekan --help` or `wekan <COMMAND> --help` for the complete command syntax.
Detailed behavior is documented in:

- [Profiles and configuration](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/config.md)
- [Authentication](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/auth.md)
- [Checklists and checklist items](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/checklists.md)
- [Custom fields and dropdown items](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/custom-fields.md)
- [Native board, card, and attachment exports](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/exports.md)
- [Attachments and file transfers](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/attachments.md)
- [Card dependencies](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/dependencies.md)
- [Board automation rules](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/rules.md)
- [Board integrations and webhook subscriptions](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/integrations.md)
- [Bulk cards and labels](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/bulk-cards-and-labels.md)
- [Board and card memberships](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/memberships.md)
- [Global administrator settings](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/admin-settings.md)
- [Administrator domains and feature controls](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/admin.md)
- [Raw API requests](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/api.md)
- [Agent output and error contract](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/agent-contract.md)
- [Command behavior and complete inventory](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/command-contract.md)
- [API-to-CLI coverage](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/api-coverage.md)
- [Local Wekan development stack](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/local-wekan.md)

## Development and security

See [CONTRIBUTING.md](https://github.com/AhmedLukman/wekan-cli/blob/main/CONTRIBUTING.md)
for development checks and API contract rules. Report vulnerabilities privately
as described in
[SECURITY.md](https://github.com/AhmedLukman/wekan-cli/blob/main/SECURITY.md).

Wekan CLI is licensed under the
[Apache License 2.0](https://github.com/AhmedLukman/wekan-cli/blob/main/LICENSE).
