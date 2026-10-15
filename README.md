# Wekan CLI

Wekan CLI is an agent-first, human-friendly command-line client for Wekan. It
provides deterministic commands, stable JSON output, explicit errors, native
credential storage, and confirmation gates for destructive operations.

Version 0.13.0 targets **Wekan v11.06 exactly**. Other Wekan versions are not
currently supported or tested.

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

## Command coverage

The CLI includes profile and authentication management plus typed commands for
users, boards, lists, swimlanes, cards, comments, checklists, checklist items,
custom-field definitions, dropdown items, attachments, card dependencies,
board integrations, and automation rules. It implements 111 of the 143 operations
in the corrected Wekan v11.06 contract,
including dedicated card archive/restore, custom-field assignment, custom-field
lookup, all 12 attachment operations, all five dependency operations, and all
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

`integration` commands manage board webhooks and their activity subscriptions,
including token input through stdin and explicit unconfirmed mutation output;
see [integrations](docs/integrations.md).
`board card-settings get/update` exposes card display controls and aging thresholds;
`board domain list/add/remove` exposes native sharing endpoints. Domain listing
requires a site administrator; valid sharing writes preserve Wekan v11.06's
`not-authorized` failure. See [board settings and sharing](docs/boards.md#card-settings-and-aging).
Dependency commands support board
and card listings, create-or-update, nullable metadata updates, and confirmed
removal with explicit acknowledgment output. Attachment transfers support files
and stdin/stdout; pinned Wekan
upload/copy/move failures and ignored list selectors remain explicitly documented.
Operations without a dedicated command remain available through the same-origin
`wekan api request` escape hatch.

Run `wekan --help` or `wekan <COMMAND> --help` for the complete command syntax.
Detailed behavior is documented in:

- [Profiles and configuration](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/config.md)
- [Authentication](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/auth.md)
- [Checklists and checklist items](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/checklists.md)
- [Custom fields and dropdown items](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/custom-fields.md)
- [Attachments and file transfers](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/attachments.md)
- [Card dependencies](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/dependencies.md)
- [Board automation rules](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/rules.md)
- [Board integrations and webhook subscriptions](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/integrations.md)
- [Bulk cards and labels](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/bulk-cards-and-labels.md)
- [Board and card memberships](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/memberships.md)
- [Raw API requests](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/api.md)
- [Agent output and error contract](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/agent-contract.md)
- [API-to-CLI coverage](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/api-coverage.md)
- [Local Wekan development stack](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/local-wekan.md)

## Development and security

See [CONTRIBUTING.md](https://github.com/AhmedLukman/wekan-cli/blob/main/CONTRIBUTING.md)
for development checks and API contract rules. Report vulnerabilities privately
as described in
[SECURITY.md](https://github.com/AhmedLukman/wekan-cli/blob/main/SECURITY.md).

Wekan CLI is licensed under the
[Apache License 2.0](https://github.com/AhmedLukman/wekan-cli/blob/main/LICENSE).
