# Releasing Wekan CLI

This runbook applies to v1.0.0 and later releases. Examples use `1.0.0`;
substitute the version being released. They describe release preparation and
publication, not the current published version.

Publishing to crates.io reserves that version permanently; it cannot be
overwritten. Release preparation stops before creating or pushing the release
tag, creating a GitHub Release, publishing a crate, or changing GitHub or
crates.io settings. Those actions require an explicit release decision after
review and successful verification.

## 1. Prepare the release change

In the release-preparation pull request:

- Set the package version in `Cargo.toml` and its `Cargo.lock` entry to the
  chosen release version.
- Move pending `CHANGELOG.md` entries into a section for that version with the
  actual release date, and retain an empty `Unreleased` section.
- Update the displayed CLI version in `README.md` and review the supported
  release policy in [SECURITY.md](SECURITY.md).
- Review the documented command, JSON, error, exit-status, and configuration
  contracts against the [compatibility policy](README.md#compatibility).
- Keep Wekan compatibility explicit. The current baseline is Wekan v12.57;
  a CLI major version does not establish support for other Wekan versions.

Require green `CI`, `Wekan E2E`, and all five `Release` pull-request artifact
builds before merging. The configured targets are Windows x64, macOS Intel
and Apple Silicon, and Linux x64 and ARM64; see
[dist-workspace.toml](dist-workspace.toml).

## 2. Verify the final release commit

After merging, work from an up-to-date, clean `main` whose local and remote
commits match:

```console
git fetch origin
git status --short --branch
git rev-list --left-right --count origin/main...HEAD
git rev-parse HEAD
```

The worktree must be clean and the left/right counts must both be zero.
Record the full commit SHA as the release candidate. Confirm that
`Cargo.toml`, the package entry in `Cargo.lock`, and `CHANGELOG.md` agree on
the chosen version and that the release date is correct.

Use the pinned Rust toolchain, cargo-deny, and cargo-dist 0.32.0. Run the local
release checks from that commit:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo build --release --locked
cargo publish --dry-run --locked
dist plan --tag v1.0.0
```

Run `target/release/wekan --version` (`target/release/wekan.exe` on Windows)
and confirm it prints `wekan 1.0.0` for this example. Also check `--help` and
`--output json skill list`; these need no server or credentials.

Regenerate and validate the corrected API contract and its HTML reference
using the [contribution instructions](CONTRIBUTING.md#api-changes). Only the
upstream YAML and correction overlay are tracked; corrected YAML and HTML are
ignored generated outputs. Set `WEKAN_CORRECTED_SPEC` to the freshly generated
YAML, then run every contract test, including the generated-contract checks:

```console
cargo test --locked --test contract -- --include-ignored
```

Every compiled live suite selected by the
[E2E matrix](.github/workflows/e2e.yml) must pass against fresh isolated Wekan
v12.57 stacks (currently 32 suites). Ordinary `cargo test` skips these suites;
its E2E inventory tests only check that the workflow selects them all. Follow
the [local Wekan guide](docs/local-wekan.md) when running suites locally.

Both GitHub release assets and crates.io publication require successful `CI`
and `Wekan E2E` workflow runs for the **exact tagged commit**. The
[release gate](.github/workflows/verify-release-tag.yml) accepts push and manual
runs from this repository and checks the newest eligible run of each workflow.
Pull-request runs alone do not satisfy it. Missing, unfinished, failed,
cancelled, or skipped runs block publication, even if an earlier run passed.

Check both workflows on the recorded `main` SHA after merging. If a run is
missing, dispatch that workflow manually on `main` and confirm its SHA matches
the candidate. This includes documentation-only release commits that do not
trigger the path-filtered E2E workflow. Any further commit requires verification
on the new candidate SHA before tagging.

## 3. Run the native Windows credential-vault test

Run this additional check on Windows at the candidate commit. Use a fresh
PowerShell session, an unused local port, and a unique Compose project. The
test mutates only that isolated Wekan stack and its temporary credential entry.

```powershell
$vaultProject = "wekan-release-vault-$([guid]::NewGuid().ToString('N'))"
$env:WEKAN_PORT = "3191"
docker compose -p $vaultProject up -d --wait
```

After the stack starts successfully:

```powershell
$env:WEKAN_OS_E2E_URL = "http://localhost:3191"
cargo test --locked --test e2e auth::os_backed_cross_process_auth_mutations_are_serialized_and_hardened -- --ignored --exact --nocapture
```

If startup or the test fails, retain the stack long enough to inspect
`docker compose -p $vaultProject logs`; resolve the failure before publishing.
After capturing any failure evidence, or after a successful run, remove this
test's stack and volumes:

```powershell
docker compose -p $vaultProject down --volumes
Remove-Item Env:WEKAN_OS_E2E_URL -ErrorAction SilentlyContinue
Remove-Item Env:WEKAN_PORT
```

## 4. Verify publication configuration

The crate already exists. Normal releases use
[publish-crate.yml](.github/workflows/publish-crate.yml) with a short-lived
OIDC credential from crates.io Trusted Publishing. Confirm that the GitHub
environment and existing Trusted Publisher match the workflow:

| Setting | Value |
| --- | --- |
| Repository owner | `AhmedLukman` |
| Repository | `wekan-cli` |
| Workflow | `publish-crate.yml` |
| Environment | `crates-io` |

Resolve any environment approvals or publisher configuration issues before
tagging. The normal release path uses the workflow rather than a manual
`cargo login` or `cargo publish`.

Check the version-specific registry endpoint before publication; for this
example, <https://crates.io/api/v1/crates/wekan-cli/1.0.0>. HTTP 404 means that
version is unused. HTTP 200 means it already exists; investigate and select an
unused version for a new release. Other responses leave registry state
unverified. The publication workflow repeats this check and skips an existing
version when rerun.

## 5. Tag and monitor publication

After the release decision, recheck that `HEAD` is the recorded, verified
candidate and the worktree is clean. Create one annotated tag at that commit
and push only that tag:

```console
git tag -a v1.0.0 -m "Wekan CLI v1.0.0"
git push origin v1.0.0
```

The tag must be `v` followed by the package version. Pushing it starts two
independent workflows:

- `Publish crate` verifies the tag, registry state, and exact-commit CI/E2E
  results, then publishes through Trusted Publishing when the version is unused.
- `Release` builds the platform artifacts and installers, checks the same
  CI/E2E gate, and publishes the GitHub release assets.

Confirm both workflows succeed. The GitHub release must contain five platform
archives, shell and PowerShell installers, `sha256.sum`, per-archive `.sha256`
files, the source archive, the dist manifest, and GitHub artifact attestations.
The binaries are not signed with Apple or Windows publisher certificates;
checksums and build provenance are the available verification mechanisms.

Do not move or recreate a published tag. If publication is blocked by a missing
CI/E2E run, run that workflow on the tagged commit, wait for success, then rerun
the blocked publication workflow without moving the tag. Rerun transient
failures only after checking whether crates.io or GitHub publication already
completed. A source or configuration defect requires a new version and release
commit; never blindly repeat a manual publication after an uncertain result.

## 6. Verify the published release

Confirm the chosen version is available on both crates.io and GitHub. Compare
the release notes with `CHANGELOG.md` and verify that the published crate and
GitHub tag correspond to the recorded release commit.

Verify downloaded checksums and GitHub attestations before running the
artifacts:

```console
gh attestation verify PATH-TO-ASSET -R AhmedLukman/wekan-cli
```

Test the shell installer on Linux or macOS and the PowerShell installer on
Windows using the commands in [README.md](README.md#install). Confirm that
each installs the chosen version. Smoke-test each platform archive on its
target platform as well.

Install the published crate into a temporary root and verify it independently
of the repository build. For example, on Windows:

```powershell
$installRoot = Join-Path $env:TEMP "wekan-cli-install-$([guid]::NewGuid().ToString('N'))"
cargo install wekan-cli --version 1.0.0 --locked --root $installRoot
& (Join-Path $installRoot "bin/wekan.exe") --version
& (Join-Path $installRoot "bin/wekan.exe") --help
& (Join-Path $installRoot "bin/wekan.exe") --output json skill list
```

Every installed binary must report the chosen version, display help, and
return valid JSON from `skill list`. Also run an authenticated JSON read, such
as `--output json board list`, against a disposable Wekan v12.57 stack. Use
temporary profiles and credentials for this check.
