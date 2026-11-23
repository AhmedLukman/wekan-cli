# Local Wekan development stack

The root [`compose.yaml`](../compose.yaml) runs a reproducible Wekan `v13.28`
instance for local CLI development and integration testing. The container images
are pinned by both version tag and digest so every developer runs the same
versions.

This stack is development infrastructure, not a production deployment.

## Services

| Service | Purpose |
| --- | --- |
| `mongodb` | Runs MongoDB `7.0.39` as the single-node `rs0` replica set required by Wekan's reactive data features. Its database is stored in the `wekan-db-data` volume and is not exposed to the host. |
| `mongodb-init` | Initializes the replica set when needed, then waits until MongoDB elects a writable primary. It exits after initialization succeeds. |
| `wekan` | Runs Wekan `v13.28` with its REST API enabled. It starts only after MongoDB is healthy and replica-set initialization succeeds. Uploaded files are stored in the `wekan-files` volume. |

Wekan is bound to `127.0.0.1`, so it is available only from the local machine.
The default URL is <http://localhost:3000>. Wekan uses SockJS for DDP, including
the raw `/websocket` endpoint used to create UI-originated test fixtures.

All services have automatic restarts disabled. After Docker or the host restarts,
start the stack again with the command below.

## Start the stack

Docker with the Compose plugin is required. From the repository root, run:

```console
docker compose up -d --wait
```

Check the service state or follow the Wekan logs with:

```console
docker compose ps
docker compose logs -f wekan
```

## Use a different port

Set `WEKAN_PORT` before starting the stack if port `3000` is already in use.
For example, in PowerShell:

```powershell
$env:WEKAN_PORT = "3001"
docker compose up -d --wait
```

The instance will then be available at <http://localhost:3001>. Use the same
`WEKAN_PORT` value for later Compose commands in that shell so Compose resolves
the configuration consistently.

## Run isolated stacks in parallel

Commands without a project-name override use the default `wekan-cli-dev`
project and therefore operate on the same containers and data. To run multiple
independent stacks, give each one a unique Compose project name with `-p` and a
unique `WEKAN_PORT`. Run each block in a separate terminal or process:

```powershell
# Stack 1
$env:WEKAN_PORT = "3101"
docker compose -p wekan-test-1 up -d --wait
```

```powershell
# Stack 2
$env:WEKAN_PORT = "3102"
docker compose -p wekan-test-2 up -d --wait
```

The stacks are available at <http://localhost:3101> and
<http://localhost:3102>, respectively. Each project has its own MongoDB and file
volumes, network, and containers. Wekan users and login credentials, boards,
cards, settings, and uploaded files are therefore independent between stacks.
The same username can exist in both stacks as two unrelated accounts. Only the
downloaded container images are shared.

This isolation supports one stack per parallel integration-test worker or CI
shard. Tests using the same stack still share its database and should create
unique test data.

The authentication lifecycle test seeds notification fixtures in its newly
registered account through the Compose `mongodb` service. Run it from the
repository root and set `COMPOSE_PROJECT_NAME` to match the isolated stack:

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-test-1"
$env:WEKAN_E2E_URL = "http://localhost:3101"
cargo test --test e2e auth::authentication_flow_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

The ignored core-board lifecycle test requires a fresh isolated stack. Point it
at that stack and run only the dedicated test:

```powershell
$env:WEKAN_BOARD_E2E_URL = "http://localhost:3101"
cargo test --test e2e complete_board_lifecycle_matches_wekan_v13_28 -- --ignored --nocapture
```

Adding `--volumes` deletes only the selected project's database and uploaded
files. Omitting `--volumes` preserves them for the next start with that project
name.

The dedicated membership lifecycle requires a fresh isolated stack and
exercises board roles, removals/reactivation, and card member/assignee merges.
Linked-card regressions independently read the source and link after each
addition/removal to verify Wekan's returned arrays describe the link:

```powershell
$env:WEKAN_MEMBERSHIP_E2E_URL = "http://localhost:3101"
cargo test --locked --test e2e memberships::complete_membership_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

## Version-specific compatibility suites

The [live test matrix](../.github/workflows/e2e.yml) lists every suite and its server URL variable. Start a fresh Compose project for each suite. From that project, set the indicated variable to its URL and run `cargo test --locked --test e2e <TEST> -- --ignored --exact --nocapture`.

| Coverage | Server URL variable | Test |
| --- | --- | --- |
| Native API extensions | `WEKAN_UPGRADE_E2E_URL` | `upgrade::native_api_extensions_complete_lifecycle` |
| Resource fields and parent-card creation | `WEKAN_METADATA_E2E_URL` | `resource_metadata::native_resource_fields_and_parent_creation` |
| Import, export, staged upload and chart formats | `WEKAN_CATALOG_E2E_URL` | `catalog::every_native_import_export_and_chart_format_matches_wekan_v13_28` |

## Native credential-store authentication races

This suite runs production CLI subprocesses against a dedicated stack and the
native credential store. It checks concurrent registration, login, logout,
credential replacement, and secret handling. CI includes it in its own matrix
entry. The default `cargo test --locked --test e2e` run only validates that the
matrix selects every compiled live test exactly once; it does not start a stack
or execute ignored tests.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-native-auth-tests"
$env:WEKAN_PORT = "3130"
docker compose up -d --wait
$env:WEKAN_OS_E2E_URL = "http://localhost:3130"
cargo test --locked --test e2e auth::os_backed_cross_process_auth_mutations_are_serialized_and_hardened -- --ignored --exact --nocapture

docker compose down --volumes
```

## Copy, move, and remaining card reads

The suite verifies board, card, list, and swimlane transfers on fresh fixtures.
It reads copied records, downloads copied attachment bytes, and checks sibling
order and destination state after each move. Set `COMPOSE_PROJECT_NAME` to
the stack containing the fixtures.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-test-1"
$env:WEKAN_TRANSFER_E2E_URL = "http://localhost:3101"
cargo test --locked --test e2e transfers::complete_copy_move_and_card_read_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

## Board card settings and domain sharing

The dedicated lifecycle covers all 124 settings, partial updates, stored numeric
values, permission boundaries, and native domain reads/writes. It requires a
fresh isolated stack and MongoDB fixtures for existing sharing records, since
Wekan returns HTTP 500 when an administrator adds a domain or removes an existing one.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-test-1"
$env:WEKAN_BOARD_SETTINGS_E2E_URL = "http://localhost:3101"
cargo test --locked --test e2e board_settings::complete_board_settings_and_domain_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

## Stop or reset the stack

Stop and remove the containers while preserving MongoDB data and uploaded files:

```console
docker compose down
```

To delete all local Wekan data and return to a fresh installation, remove the
containers and their named volumes:

```console
docker compose down --volumes
```

The volume-removal command permanently deletes the local database and uploaded
files. The next start will require Wekan's initial account setup again.

## Attachment lifecycle

The attachment suite covers metadata, file replacement, uploads, downloads,
deletion, resource scoping, and raw output. It checks both filesystem and
GridFS files on a dedicated stack. Set the Compose project name so the suite
can inspect the stored bytes.

```powershell
$env:WEKAN_PORT = "3107"
$env:COMPOSE_PROJECT_NAME = "wekan-attachment-tests"
docker compose up -d --wait
$env:WEKAN_ATTACHMENT_E2E_URL = "http://localhost:3107"
cargo test --locked --test e2e attachments::complete_attachment_flow_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

Use the matching Compose project and URL; this suite mutates the isolated stack.
It is included in the Wekan E2E workflow matrix. Remove that disposable stack
and its volumes when finished with `docker compose down --volumes`.

## User API token lifecycle

Run the token suite on a fresh isolated stack. It exercises production CLI
parsing, stdin, native credential storage, strict response decoding, human/JSON
output, issued-token authentication, raw/hash matching, missing users/tokens,
admin denials, delete-all, self-revocation, and subsequent password login.
MongoDB readbacks expose only token counts and field types.

```powershell
$env:WEKAN_PORT = "3183"
$env:COMPOSE_PROJECT_NAME = "wekan-token-tests"
docker compose up -d --wait
$env:WEKAN_TOKEN_E2E_URL = "http://localhost:3183"
cargo test --locked --test e2e user_tokens::complete_user_token_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

The suite is included in the E2E workflow matrix and cleans up its local vault
entries. Remove the disposable stack with `docker compose down --volumes`.

## Troubleshooting

Inspect the state and logs for all three services:

```console
docker compose ps --all
docker compose logs mongodb mongodb-init wekan
```

If Wekan is unhealthy, first confirm that `mongodb-init` completed successfully
and that `mongodb` reports healthy. If startup reports that the host port is in
use, stop the process using that port or restart the stack with another
`WEKAN_PORT` value.

## Checklist and checklist item lifecycle

Run the dedicated ignored suite against a fresh isolated stack. It covers all eight
operations, exact input forms, defaults and coercion, scoped failures, role
permissions, rejection of invalid items, deletion behavior, and reads after a cross-board
REST card move. It also creates checklists
and items through the UI's DDP collection methods and verifies creator IDs in
typed REST reads and human/JSON output. The DDP fixture runs in Rust and connects
to the isolated stack's WebSocket endpoint using `WEKAN_CHECKLIST_E2E_URL` (an
HTTP URL). MongoDB fixtures still use Docker Compose; the CLI continues to use
REST. Use the same Compose project name for the fixtures and the matching server
URL:

```powershell
$env:WEKAN_PORT = "3103"
$env:COMPOSE_PROJECT_NAME = "wekan-checklist-tests"
docker compose up -d --wait
$env:WEKAN_CHECKLIST_E2E_URL = "http://localhost:3103"
cargo test --locked --test e2e checklists::complete_checklist_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

This suite is also registered in the Wekan E2E workflow's isolated-stack matrix.

## Custom-field lifecycle

The dedicated suite covers all ten custom-field operations, nullable and omitted
settings with exact request preservation, dropdown filtering, exact card JSON
inputs and cleaned responses, string lookup, role
gates, shared deletion, and partial-update failures.
Fixtures use only the isolated project's data; typed response decoding remains
strict. The suite is registered in the E2E workflow matrix.
Regression cases verify deletion output for real, repeated, missing,
and wrong-board targets, native HTTP 404 on missing reads, and query
encoding near dot segments. Dot-only lookup inputs are rejected locally.
Dropdown regression cases verify Wekan's acceptance and trimming of supplied
dot-only IDs, local rejection of unaddressable selectors/settings, and successful
update/deletion of IDs containing encoded slashes, percent signs, and dot text.
They also verify that U+0085 IDs remain readable and deleting an option surrounded
by U+0085 leaves its plain-ID counterpart intact.

```powershell
$env:WEKAN_PORT = "3114"
$env:COMPOSE_PROJECT_NAME = "wekan-custom-field-tests"
docker compose up -d --wait
$env:WEKAN_CUSTOM_FIELD_E2E_URL = "http://localhost:3114"
cargo test --locked --test e2e custom_fields::complete_custom_field_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

## Integration and webhook-subscription lifecycle

The integration suite covers all seven operations, defaults, token stdin and
clearing, replacement/addition/removal of subscriptions, duplicates, role gates,
missing and wrongly scoped integrations, URL rejection, and partial updates.
It uses literal public-IP URLs for validation and disables integrations before
generating board events; it does not test outbound delivery. Token storage is
checked through the isolated MongoDB fixture, while typed reads exclude tokens.
Use the same Compose project for fixture checks and the server URL:

```powershell
$env:WEKAN_PORT = "3107"
$env:COMPOSE_PROJECT_NAME = "wekan-integration-tests"
docker compose up -d --wait
$env:WEKAN_INTEGRATION_E2E_URL = "http://localhost:3107"
cargo test --locked --test e2e integrations::complete_integration_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

The suite is registered in the Wekan E2E workflow's isolated-stack matrix.

## Card dependency lifecycle

The dependency suite covers listing, creation, updates, deletion, relation types,
optional metadata, missing resources, and board permissions. It checks results
through the production client and native card reads on a dedicated stack.

```powershell
$env:WEKAN_PORT = "3011"
$env:COMPOSE_PROJECT_NAME = "wekan-dependency-tests"
docker compose up -d --wait
$env:WEKAN_DEPENDENCY_E2E_URL = "http://localhost:3011"
cargo test --locked --test e2e dependencies::complete_dependency_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```

## Bulk card and label lifecycle

The suite verifies card creation and deletion in batches, label reuse and assignment,
and the resulting card, comment, and attachment records. Its Compose fixtures
belong to the same isolated project as the test server.

```powershell
$env:WEKAN_PORT = "3116"
$env:COMPOSE_PROJECT_NAME = "wekan-bulk-label-tests"
docker compose up -d --wait
$env:WEKAN_BULK_CARD_E2E_URL = "http://localhost:3116"
cargo test --locked --test e2e bulk_cards::complete_bulk_card_and_label_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
```


## Board automation rule lifecycle

The rule suite covers all five REST endpoints, every built-in payload field and
kind, permissions, wildcard normalization, shallow merging, null/absent fields,
strict rejection of custom payload fields, dangling references, and referenced
document deletion. It proves move-to/add-member and move-away/remove-member
execution and checks scheduled-rule execution and stored last-run metadata. Button
persistence and REST destination rebinding are checked without DDP workarounds.
Fixture checks require the same Compose project as the server URL. The suite
also runs in the isolated-stack CI matrix.
Regression cases reject invalid `lastRunAt` input without changing stored rules
and round-trip 2–3 MiB definitions through create/update and both read commands.

```powershell
$env:WEKAN_PORT = "3117"
$env:COMPOSE_PROJECT_NAME = "wekan-rule-tests"
docker compose up -d --wait
$env:WEKAN_RULE_E2E_URL = "http://localhost:3117"
cargo test --locked --test e2e rules::complete_rule_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

## Global administrator settings lifecycle

Use a fresh disposable stack; this suite changes global settings and finishes
by exercising the missing-global-document branches. It checks every supported
field, native nested replacement/defaults, cloud merging and secret preservation,
strict sparse/unknown-field responses, site-admin permissions, and the production
binary's JSON/credential stdin paths. Cloud providers use dummy credentials and
remain disabled; external cloud transfers are not tested.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-admin-settings-test"
$env:WEKAN_PORT = "3118"
docker compose up -d --wait
$env:WEKAN_ADMIN_SETTINGS_E2E_URL = "http://localhost:3118"
cargo test --locked --test e2e admin_settings::complete_admin_settings_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

Keep the same Compose project environment for the test's fixture operations.
The suite is registered in the isolated-stack Wekan E2E workflow matrix.

## Administrator domain and feature lifecycle

Use a separate fresh stack for the seven domain/organization/team operations.
The suite seeds fixture-only organization, team, and email records in MongoDB,
then exercises production REST handlers and the binary. It checks all feature
booleans, sparse and complete documents, native bulk counts, administrator
permissions, missing IDs, strict decoding after writes, and confirmation gates.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-admin-people-test"
$env:WEKAN_PORT = "3119"
docker compose up -d --wait
$env:WEKAN_ADMIN_PEOPLE_E2E_URL = "http://localhost:3119"
cargo test --locked --test e2e admin_people::complete_admin_people_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

Keep the same Compose project environment for fixture operations. This lifecycle
and the administrator-settings regression each have an isolated CI matrix entry.

## Native export lifecycle

The dedicated suite covers all ten native operations and every board format,
scoped/card/attachment output, field and locale controls, public/private access,
strict decoding, and pinned ZIP and binary-error behavior. XLSX entries are
opened and CRC-checked; PDF structure and native JSON/CSV contents are inspected.
Filesystem attachment fixtures use the same Compose project as the server.
Native CSV, Trello, and Wekan imports also verify export of their persisted
import history, including optional source IDs and URLs.
UI/DDP and REST-created checklists are independently read from persisted state
and exported to verify creator ID preservation and omission.
Native DDP watch/unwatch updates verify omitted, populated, and empty watcher
arrays on cards and subtasks against independently read persisted state.
The suite is included in the isolated-stack CI matrix.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-export-tests"
$env:WEKAN_PORT = "3120"
docker compose up -d --wait
$env:WEKAN_EXPORT_E2E_URL = "http://localhost:3120"
cargo test --locked --test e2e exports::complete_export_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

## Import lifecycle

The import suite exercises board, Trello, and iCalendar import operations
against an isolated Wekan v13.28 stack. It verifies source document wrapping,
CSV/XLSX input, member mapping, duplicate imports, ZIP scopes and field selection,
and ICS event contents/dates against independently queried persisted records.
It also records native permission/missing-target failures and verifies that
ZIP attachment import completes, preserves its bytes and cleans temporary
archives. The catalog suite covers all 85 sources and staged XLSX uploads. The export suite verifies
native ZIP round trips and new-board import outcomes. All suites
are included in the isolated-stack CI matrix.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-import-tests"
$env:WEKAN_PORT = "3121"
docker compose up -d --wait
$env:WEKAN_IMPORT_E2E_URL = "http://localhost:3121"
cargo test --locked --test e2e imports::complete_import_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

## Trello HTTP import lifecycle

The dedicated Trello HTTP import suite uses another fresh isolated stack:

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-trello-tests"
$env:WEKAN_PORT = "3127"
docker compose up -d --wait
$env:WEKAN_TRELLO_IMPORT_E2E_URL = "http://localhost:3127"
cargo test --locked --test e2e trello_import::complete_trello_import_lifecycle_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```

It verifies JSON member mapping, all four login-token authentication forms,
multi-board ZIP and attachment behavior, partial/all-board failures, and
independent persisted-state readback. The CI matrix runs it in its own stack.

## Public probes and legacy attachment links

The public-probe suite verifies schema-upgrade JSON output, all three anonymous
authentication OPTIONS routes, and operation without a saved profile or login.
The attachment lifecycle also covers empty/populated legacy board link lists,
filesystem/GridFS fixtures, background selector omission, normal-member access,
and resource scoping.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-coverage-tests"
$env:WEKAN_PORT = "3122"
docker compose up -d --wait
$env:WEKAN_PUBLIC_PROBE_E2E_URL = "http://localhost:3122"
cargo test --locked --test e2e public_probes::public_probes_match_wekan_v13_28 -- --ignored --exact --nocapture
$env:WEKAN_ATTACHMENT_E2E_URL = "http://localhost:3122"
cargo test --locked --test e2e attachments::complete_attachment_flow_matches_wekan_v13_28 -- --ignored --exact --nocapture
docker compose down --volumes
```
