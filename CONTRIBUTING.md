# Contributing to Wekan CLI

Thank you for contributing to Wekan CLI.

Use the [issue tracker][issues] for bugs and feature requests. Report suspected
vulnerabilities privately as described in [`SECURITY.md`](SECURITY.md), never
in a public issue or pull request.

## Before you start

Open a pull request directly for small fixes and documentation updates. For
architectural work or changes to Wekan compatibility, open an issue first to
agree on the approach.

## Development setup

Install Git and [Rustup][rustup]. The pinned
[`rust-toolchain.toml`](rust-toolchain.toml) provides Rust 1.88.0, Rustfmt, and
Clippy. Docker Compose is needed only for live Wekan tests.

```console
git clone https://github.com/<YOUR-USER>/wekan-cli.git
cd wekan-cli
cargo build
```

## Making changes

Create a focused branch from the latest `main`. Include relevant tests and
documentation, and follow the
[repository folder structure guide](docs/folder-structure.md) when adding or
moving code.

Add user-visible features, fixes, and behavior changes under `Unreleased` in
[`CHANGELOG.md`](CHANGELOG.md). Documentation-only changes and internal
refactors do not need an entry.

## Checks

Run these commands before submitting a pull request:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

If `Cargo.toml` or `Cargo.lock` changed, also run `cargo deny check`.

Live tests are ignored by default. Follow the
[local Wekan stack guide](docs/local-wekan.md) and use a fresh or dedicated
Wekan v11.06 instance for tests that mutate data. Never run destructive tests
against a real instance.

## API changes

API commands target Wekan v11.06. Use the corrected
[`wekan.corrected.yml`](spec/wekan.corrected.yml) contract and verify any
discrepancy against the matching [Wekan source][wekan-source] and a live server.
Response objects, including nested objects, must use
`#[serde(deny_unknown_fields)]`.

Keep tests and documentation synchronized. Record verified contract corrections
only in [`wekan.overlay.yml`](spec/wekan.overlay.yml). Do not edit generated
artifacts directly.

The commands below require the Speakeasy `openapi` CLI on `PATH`. Generating
the HTML reference also requires Node.js and npm.

Validate the overlay and regenerate the corrected contract:

```console
openapi overlay validate spec/wekan.overlay.yml
openapi overlay apply --schema spec/wekan.yml --overlay spec/wekan.overlay.yml --out spec/wekan.corrected.yml
openapi swagger validate spec/wekan.corrected.yml
```

Regenerate the human-readable reference with Redocly:

```console
npx --yes @redocly/cli@2.46.0 build-docs spec/wekan.corrected.yml --output spec/wekan.corrected.html --title "WeKan REST API v11.06 (Corrected)"
```

## Pull requests

Target `main` and use [Conventional Commits][conventional-commits]. Explain the
problem, the chosen solution, and how it was tested. Before requesting review,
check that:

- Required checks pass.
- Documentation and the changelog are updated when applicable.
- API coverage and generated artifacts are updated when applicable.
- Changes follow the repository's existing architecture and file organization.

[conventional-commits]: https://www.conventionalcommits.org/en/v1.0.0/
[issues]: https://github.com/AhmedLukman/wekan-cli/issues/new
[rustup]: https://rustup.rs/
[wekan-source]: https://github.com/wekan/wekan/tree/v11.06
