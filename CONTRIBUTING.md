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

CLI subprocess tests use temporary configuration directories and clear inherited
`WEKAN_URL` and `WEKAN_PROFILE` values. Use the shared helpers in
`tests/cli/support.rs`; set environment variables explicitly when testing
precedence. CI runs the suite with deliberately invalid inherited configuration
to keep this isolation checked on all three platforms.

Live tests are ignored by default. Follow the
[local Wekan stack guide](docs/local-wekan.md) and use a fresh or dedicated
Wekan v12.57 instance for tests that mutate data. Never run destructive tests
against a real instance.

The default E2E test run checks the workflow matrix against the compiled binary's
ignored test inventory without contacting Wekan. Add or rename the matching
`.github/workflows/e2e.yml` entry whenever a live test changes. Missing, stale,
and duplicate selections fail before CI starts services.

## API changes

API commands target Wekan v12.57. Generate the corrected
`spec/wekan.corrected.yml` contract using the commands below and verify any
discrepancy against the matching [Wekan source][wekan-source] and a live server.
Response objects, including nested objects, must use
`#[serde(deny_unknown_fields)]`.

Keep tests and documentation synchronized. Record verified contract corrections
only in [`wekan.overlay.yml`](spec/wekan.overlay.yml). The only committed API
reference files are the upstream [`wekan.yml`](spec/wekan.yml) and its overlay.
The corrected YAML and all HTML references under `spec/` are ignored local
outputs. Generate them on demand; do not edit or commit them.

The commands below require Speakeasy OpenAPI CLI 1.24.1 (`openapi`) on `PATH`.
Generating the HTML references also requires Node.js and npm.

Validate the overlay and regenerate the corrected contract:

```console
openapi overlay validate spec/wekan.overlay.yml
openapi overlay apply --schema spec/wekan.yml --overlay spec/wekan.overlay.yml --out spec/wekan.corrected.yml
openapi swagger validate spec/wekan.corrected.yml
```

Set `WEKAN_CORRECTED_SPEC` to the freshly generated specification's path, then
run `cargo test --locked --test contract -- --include-ignored`. This compares
every documented method, path, and operation ID against the generated contract.
The default test suite also checks coverage totals, CLI command paths,
positional arguments, option names, and value placeholders against Clap. CI
performs both checks as part of API contract validation.

Optionally generate the corrected human-readable reference with Redocly:

```console
npx --yes @redocly/cli@2.46.0 build-docs spec/wekan.corrected.yml --output spec/wekan.corrected.html --title "WeKan REST API v12.57 (Corrected)"
```

To render the upstream specification without corrections instead:

```console
npx --yes @redocly/cli@2.46.0 build-docs spec/wekan.yml --output spec/wekan.html --title "WeKan REST API v12.57 (Upstream)"
```

Open the generated HTML files locally in a browser. CI generates and validates
the corrected specification and builds its HTML reference in a temporary
directory, without requiring committed generated files. Rust builds and tests
do not require these generated references.

## Pull requests

Target `main` and use [Conventional Commits][conventional-commits]. Explain the
problem, the chosen solution, and how it was tested. Before requesting review,
check that:

- Required checks pass.
- Documentation and the changelog are updated when applicable.
- The API overlay and coverage are updated, and corrected references generate
  and validate successfully when applicable.
- Changes follow the repository's existing architecture and file organization.

[conventional-commits]: https://www.conventionalcommits.org/en/v1.0.0/
[issues]: https://github.com/AhmedLukman/wekan-cli/issues/new
[rustup]: https://rustup.rs/
[wekan-source]: https://github.com/wekan/wekan/tree/v12.57
