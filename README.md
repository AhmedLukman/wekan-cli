# Wekan CLI

Wekan CLI is an agent-first, human-friendly command-line client for Wekan. It
provides deterministic commands, stable JSON output, explicit errors, native
credential storage, and confirmation gates for destructive operations.

Version 0.1.0 targets **Wekan v11.06 exactly**. Other Wekan versions are not
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

The first release includes profile and authentication management plus typed
commands for users, boards, lists, swimlanes, cards, and comments. It implements
36 of the 140 operations in the corrected Wekan v11.06 contract. Operations
without a dedicated command remain available through the same-origin
`wekan api request` escape hatch.

Run `wekan --help` or `wekan <COMMAND> --help` for the complete command syntax.
Detailed behavior is documented in:

- [Profiles and configuration](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/config.md)
- [Authentication](https://github.com/AhmedLukman/wekan-cli/blob/main/docs/auth.md)
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
