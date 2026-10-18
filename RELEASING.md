# Releasing Wekan CLI

This runbook cuts a public Wekan CLI release. Publishing to crates.io is
effectively permanent: a published version cannot be overwritten or deleted.
Do not run the publication or tag commands until the release commit has been
reviewed and every gate below is green.

Both GitHub release assets and crates.io publication require successful `CI`
and `Wekan E2E` workflow runs for the exact tagged commit. The gate accepts
push and manual runs from this repository and checks the newest eligible run
of each workflow. Missing, unfinished, failed, cancelled, or skipped runs block
publication, even if an earlier run passed. Run a missing workflow manually on
the candidate branch, wait for both workflows to pass, then rerun the blocked
publication workflow without moving the tag. This also applies to release
commits whose changes do not trigger the path-filtered E2E workflow.

## v0.1.0 stop boundary

Release preparation stops before `cargo publish`, creation or pushing of the
`v0.1.0` tag, creation of a GitHub Release, and changes to GitHub or crates.io
settings. Those actions require an explicit release decision after review.

## 1. Verify the release commit

Work from an up-to-date, clean `main` whose local and remote commits match:

```console
git fetch origin
git status --short --branch
git rev-list --left-right --count origin/main...HEAD
```

Confirm that `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and `wekan --version`
all identify `0.1.0`, and that the changelog release date is correct. Recheck
that <https://crates.io/api/v1/crates/wekan-cli> returns HTTP 404 immediately
before the first publication; crate names are allocated first-come,
first-served.

Run the local release checks:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo build --release --locked
cargo publish --dry-run --locked
dist plan --tag v0.1.0
```

Validate and regenerate the corrected API contract as described in
`AGENTS.md`. Only the upstream YAML and correction overlay are tracked;
corrected YAML and HTML are ignored generated outputs. Run the ignored
`coverage::generated_contract_matches_documented_operations` contract test
with `WEKAN_CORRECTED_SPEC` pointing to the freshly generated YAML. The
release-preparation pull request must also have green CI, all five dist
artifact builds, and every live suite selected by the E2E matrix (currently
29). Ordinary `cargo test` skips the live suites; it does not verify Wekan
compatibility. The compiled E2E inventory test guards the matrix selections.

## 2. Run the native Windows credential-vault test

Use a unique Compose project and port. This test is destructive only to that
isolated Wekan stack and its matching temporary credential entry.

```powershell
$env:WEKAN_PORT = "3191"
docker compose -p wekan-release-v0-1-0-vault up -d --wait
$env:WEKAN_OS_E2E_URL = "http://localhost:3191"
cargo test --locked --test e2e auth::os_backed_cross_process_auth_mutations_are_serialized_and_hardened -- --ignored --exact --nocapture
docker compose -p wekan-release-v0-1-0-vault down --volumes
Remove-Item Env:WEKAN_OS_E2E_URL
Remove-Item Env:WEKAN_PORT
```

If the test fails, retain the stack long enough to inspect `docker compose -p
wekan-release-v0-1-0-vault logs`; do not publish.

## 3. Publish the first crate manually

The first release must be published manually because crates.io Trusted
Publishing can only be configured after the crate exists. Create a short-lived
crates.io token with only the permissions needed for the initial publication.
Enter it through Cargo's prompt, never through a committed file or shell
argument:

```console
cargo login
cargo publish --locked
cargo logout
```

Immediately revoke the token in crates.io settings. If Cargo times out or
returns an uncertain result after upload, check the crate page and API before
retrying; never blindly repeat a publication command.

Wait for `wekan-cli` version `0.1.0` to appear in the registry, then install it
into a temporary root and smoke-test the published package:

```powershell
$installRoot = Join-Path $env:TEMP "wekan-cli-v0.1.0-install"
cargo install wekan-cli --version 0.1.0 --locked --root $installRoot
& (Join-Path $installRoot "bin/wekan.exe") --version
& (Join-Path $installRoot "bin/wekan.exe") --help
```

The version command must print `wekan 0.1.0`.

## 4. Configure future Trusted Publishing

Create a GitHub environment named `crates-io`. In the new crate's crates.io
settings, add a GitHub Trusted Publisher with:

| Setting | Value |
| --- | --- |
| Repository owner | `AhmedLukman` |
| Repository | `wekan-cli` |
| Workflow | `publish-crate.yml` |
| Environment | `crates-io` |

After verifying a later OIDC publication, enforce Trusted Publishing and
revoke any remaining crates.io API tokens. The workflow intentionally skips a
version that already exists, so the manually published v0.1.0 tag run is
idempotent.

## 5. Create the GitHub release

Create one annotated tag at the exact reviewed commit and push only that tag:

```console
git tag -a v0.1.0 -m "Wekan CLI v0.1.0"
git push origin v0.1.0
```

The generated release workflow must publish five platform archives, shell and
PowerShell installers, `sha256.sum`, per-archive `.sha256` checksum files, the
dist manifest, and GitHub artifact attestations. Do not move or recreate a
published tag. If a GitHub Actions job fails, fix only a transient external
issue by rerunning the same workflow; a source or configuration defect requires
a new version.

## 6. Post-release verification

Verify checksums and attestations, then test the shell installer on Linux or
macOS and the PowerShell installer on Windows. Each installed binary must run
`wekan --version`, `wekan --help`, and a JSON command against a disposable Wekan
v11.06 stack. Confirm that the GitHub release notes match `CHANGELOG.md` and
that both crates.io and GitHub link back to the reviewed repository commit.
