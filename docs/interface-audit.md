# Command interface audit

Verified on 2026-10-18 on Windows with Rust 1.88.0, against the digest-pinned
Wekan v11.06 Compose stack. The audited baseline is commit `9d2ac05` plus the
changes described below. The package version remains 0.21.0.

**Result: the requested interface audit passes after these corrections. No
unresolved command/JSON/error/exit-status inconsistency was found for the
pinned server.** The [command reference](command-contract.md) collects the
verified behavior and complete command inventory.

## Scope and findings

The inventory covers all 143 leaf commands in 17 families, all 144 corrected
API operations, the six local profile commands, and the raw API escape hatch.
The success dispatcher has 134 variants; the error catalogue has 28 codes,
and cancellation has 23 operation identifiers.

| Surface | Finding and resolution |
| --- | --- |
| Command syntax | Added executable tests for help and JSON usage-error streams/codes at every command/group depth, with output selection both before and after the command path. The complete leaf inventory is checked against Clap. Existing family/parser tests cover selectors, conflicts, repeated inputs, and validation. |
| JSON fields and types | Verified the typed, resource-focused results and shared envelopes against the contract tests and live suites. |
| Null versus omission/empty | Corrected blanket collection claims. Card/list watchers, user profile collections/maps, swimlane card assignees, custom-field dropdown items, and allowed MIME types have documented nullable behavior. Rule properties preserve omission versus explicit null; Trello errors are format-dependent; error details omit inapplicable properties. The command reference collects these distinctions in one table. |
| Error and exit classifications | Completed the catalogue with `batch_incomplete` and `file_io_error`. Documented the existing contextual exit mappings for `protocol_error` (4/5), `internal_error` (1/4), and `credential_not_found` (5/6). These are distinct stable signals, rather than a one-to-one mapping. Completed the confirmation/cancellation lists. |
| Output failure after a mutation | Found and fixed loss of HTTP/mutation/retry context when human/JSON raw-API success output could not be delivered. Write and flush failures now retain the known status; unsafe methods retain mutation-attempt metadata and `retry_safe: false`. The regression failed before the fix and passes for GET/POST, both formats, and both failure modes, with exact request counts. |
| Verification guidance | Corrected the nine-test assumption. The runbook requires every compiled live suite and correctly treats corrected YAML/HTML as ignored generated outputs. |

Typed Wekan objects reject unmapped fields. Raw API data and native
export formats retain their documented boundaries. Existing pinned server
defects, acknowledgements, no-ops, and partial writes remain documented behavior;
this audit adds no server-side workaround or automatic mutation retry.

## Verification evidence

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --all-targets --all-features --locked` | 693 passed: 357 library, 93 executable, 241 contract, 2 live-inventory tests |
| Both ignored generated-contract tests, with `WEKAN_CORRECTED_SPEC` set | 2 passed; operation inventory and verified corrections match |
| Speakeasy OpenAPI CLI 1.24.1 overlay validation/application and corrected Swagger validation | Passed; zero validation errors |
| Corrected HTML generation with Redocly CLI 2.46.0 | Succeeded |
| Individually selected ignored Wekan live suites | All 29 passed, each against a fresh isolated stack |
| Error catalogue comparison against `ErrorCode::as_str` | All 28 codes documented |
| Final matrix/log reconciliation | All 29 expected test names selected exactly once in the combined results; each log records one passed test and zero failures |
| Audit stack cleanup | Temporary audit containers and their Compose volumes removed |

The two ignored library tests are subprocess lock helpers exercised by their
parent tests. Live tests and generated-contract tests were run separately from
the ordinary test command, which skips them by default.

Live matrix results:

| Suite | Result |
| --- | --- |
| Public probes | Passed |
| Copy/move and card reads | Passed |
| Bulk cards and labels | Passed |
| Board/card memberships | Passed |
| Dependencies | Passed |
| Automation rules | Passed |
| Integrations | Passed |
| Raw API | Passed |
| Authentication | Passed |
| Named-profile authentication | Passed |
| Native Windows credential-store races | Passed |
| Users | Passed |
| User tokens | Passed |
| Pinned API compatibility | Passed |
| Compatibility audit regressions | Passed |
| Boards | Passed |
| Board settings and domains | Passed |
| Global admin/attachment settings | Passed |
| Admin domains and organization/team features | Passed |
| Lists | Passed |
| Swimlanes | Passed |
| Cards | Passed |
| Comments | Passed |
| Checklists/items | Passed |
| Custom fields/dropdown items | Passed |
| Exports | Passed |
| Imports | Passed |
| Trello imports | Passed |
| Attachments | Passed |

Local evidence remains under ignored `target/interface-audit/`: `offline-tests.log`,
`clippy.log`, the combined `results.json`, and per-suite logs in that directory
and its `shard2/` subdirectory. The combined result file preserves exact test
identifiers from the [CI matrix](../.github/workflows/e2e.yml).

## Verification scope

This verifies the current interface on Windows and Wekan v11.06. Cross-platform
CI, packaging, and distribution checks were not run by this audit. The
applicable verification gates are described in [RELEASING.md](../RELEASING.md).
No version bump, tag, or publication was performed.
