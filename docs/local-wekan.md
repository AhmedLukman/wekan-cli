# Local Wekan development stack

The root [`compose.yaml`](../compose.yaml) runs a reproducible Wekan `v11.06`
instance for local CLI development and integration testing. The container images
are pinned by both version tag and digest so every developer runs the same
versions.

This stack is development infrastructure, not a production deployment.

## Services

| Service | Purpose |
| --- | --- |
| `mongodb` | Runs MongoDB `7.0.39` as the single-node `rs0` replica set required by Wekan's reactive data features. Its database is stored in the `wekan-db-data` volume and is not exposed to the host. |
| `mongodb-init` | Initializes the replica set when needed, then waits until MongoDB elects a writable primary. It exits after initialization succeeds. |
| `wekan` | Runs Wekan `v11.06` with its REST API enabled. It starts only after MongoDB is healthy and replica-set initialization succeeds. Uploaded files are stored in the `wekan-files` volume. |

Wekan is bound to `127.0.0.1`, so it is available only from the local machine.
The default URL is <http://localhost:3000>.

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
cargo test --test e2e auth::authentication_flow_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

The ignored core-board lifecycle test requires a fresh isolated stack. Point it
at that stack and run only the dedicated test:

```powershell
$env:WEKAN_BOARD_E2E_URL = "http://localhost:3101"
cargo test --test e2e complete_board_lifecycle_matches_wekan_v11_06 -- --ignored --nocapture
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
cargo test --locked --test e2e memberships::complete_membership_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

## Copy, move, and remaining card reads

The dedicated lifecycle test covers all ten operations, nested copies,
position inputs, archived-card lookup, permissions, and partial move defects.
Independent list-move fixtures distinguish new destinations with matching or
nonmatching source swimlane filters from existing same-title destinations that
bypass that filter. Copy results assert unconfirmed metadata even when the
returned board/card exists and its attachments are omitted.
It independently checks every sibling's persisted sort value and polls the
asynchronous extra-default-lane insertion before verifying swimlane ordering.
Run it against an isolated stack and set `COMPOSE_PROJECT_NAME` to that stack
when using database/file fixtures:

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-test-1"
$env:WEKAN_TRANSFER_E2E_URL = "http://localhost:3101"
cargo test --locked --test e2e transfers::complete_copy_move_and_card_read_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

## Board card settings and domain sharing

The dedicated lifecycle covers all 47 settings, partial updates, stored numeric
values, permission boundaries, and native domain reads/writes. It requires a
fresh isolated stack and MongoDB fixtures for existing sharing records, since
Wekan v11.06 domain writes fail with `not-authorized`.

```powershell
$env:COMPOSE_PROJECT_NAME = "wekan-test-1"
$env:WEKAN_BOARD_SETTINGS_E2E_URL = "http://localhost:3101"
cargo test --locked --test e2e board_settings::complete_board_settings_and_domain_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
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

The attachment suite covers all 12 operations, real filesystem/GridFS bytes,
strict metadata/version decoding, downloads above 1 MiB, file replacement,
raw stdout, missing resources, deletion, ignored deeper list scopes, and the four
normal JSON POST operations' HTTP 408 responses. Success POST contracts are
mock-tested separately because the pinned server cannot execute those paths.
Deletion also polls up to 10 seconds for the seeded filesystem path and GridFS
files/chunks to disappear before board or stack cleanup can remove those bytes.
Test-only persisted fixtures seed files; the CLI continues to use REST.

```powershell
$env:WEKAN_PORT = "3107"
$env:COMPOSE_PROJECT_NAME = "wekan-attachment-tests"
docker compose up -d --wait
$env:WEKAN_ATTACHMENT_E2E_URL = "http://localhost:3107"
cargo test --locked --test e2e attachments::complete_attachment_flow_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

Use the matching Compose project and URL; this suite mutates the isolated stack.
It is included in the Wekan E2E workflow matrix. Remove that disposable stack
and its volumes when finished with `docker compose down --volumes`.

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

## Checklist and checklist-item lifecycle

Run the dedicated ignored suite against a fresh isolated stack. It covers all eight
operations, exact input forms, defaults and coercion, scoped failures, role
permissions, partial creation, deletion behavior, and reads after a cross-board
REST card move with stale board metadata. It also creates checklists
and items through the UI's DDP collection methods and verifies creator IDs in
typed REST reads and human/JSON output. The DDP fixture runs with Node.js inside
the pinned Wekan container; the CLI continues to use REST. Use the same Compose
project name for the fixtures and the matching server URL:

```powershell
$env:WEKAN_PORT = "3103"
$env:COMPOSE_PROJECT_NAME = "wekan-checklist-tests"
docker compose up -d --wait
$env:WEKAN_CHECKLIST_E2E_URL = "http://localhost:3103"
cargo test --locked --test e2e checklists::complete_checklist_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

This suite is also registered in the Wekan E2E workflow's isolated-stack matrix.

## Custom-field lifecycle

The dedicated suite covers all ten custom-field operations, nullable and omitted
settings with exact request preservation, dropdown filtering, exact card JSON
inputs and cleaned responses, string lookup, role
gates, no-op acknowledgments, shared deletion, and partial-update failures.
Fixtures use only the isolated project's data; typed response decoding remains
strict. The suite is registered in the E2E workflow matrix.
Regression cases verify unconfirmed deletion output for real, repeated, missing,
and wrong-board targets, preserved HTTP 200 metadata on missing reads, and query
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
cargo test --locked --test e2e custom_fields::complete_custom_field_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
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
cargo test --locked --test e2e integrations::complete_integration_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
docker compose down --volumes
```

The suite is registered in the Wekan E2E workflow's isolated-stack matrix.

## Card dependency lifecycle

The dependency suite covers both listing endpoints, all relation types,
create-or-update merging, partial and empty/null metadata updates, raw card reads,
confirmed removal, missing-edge acknowledgments, archived sources, dangling
targets, and board permissions. It preserves pinned HTTP 500 validation failures
and runs through strict production DTO decoding. Use an isolated stack:

```powershell
$env:WEKAN_PORT = "3011"
$env:COMPOSE_PROJECT_NAME = "wekan-dependency-tests"
docker compose up -d --wait
$env:WEKAN_DEPENDENCY_E2E_URL = "http://localhost:3011"
cargo test --locked --test e2e dependencies::complete_dependency_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
```

## Bulk card and label lifecycle

The dedicated suite covers native creation/deletion/label batches, scoped label
reads, label reuse and name-cleaning defects, role gates, assignments, duplicates,
nullable label results with independent stored-state reads,
archived/foreign/missing cards, comment deletion, retained attachment metadata
and filesystem bytes after successful card deletion, and partial creation.
Attachment fixtures use Docker Compose to seed the ordinary persisted metadata
and genuine bytes because the pinned REST upload cannot create them. Use the
same Compose project for fixtures and the matching server URL. The suite is also
registered in the isolated-stack CI matrix.

```powershell
$env:WEKAN_PORT = "3116"
$env:COMPOSE_PROJECT_NAME = "wekan-bulk-label-tests"
docker compose up -d --wait
$env:WEKAN_BULK_CARD_E2E_URL = "http://localhost:3116"
cargo test --locked --test e2e bulk_cards::complete_bulk_card_and_label_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
```


## Board automation rule lifecycle

The rule suite covers all five REST endpoints, every built-in payload field and
kind, permissions, wildcard normalization, shallow merging, null/absent fields,
strict rejection of custom payload fields, dangling references, and referenced
document deletion. It proves move-to/add-member and move-away/remove-member
execution and verifies the pinned scheduler non-execution for 90 seconds. Button
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
cargo test --locked --test e2e rules::complete_rule_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
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
cargo test --locked --test e2e admin_settings::complete_admin_settings_lifecycle_matches_wekan_v11_06 -- --ignored --exact --nocapture
docker compose down --volumes
```

Keep the same Compose project environment for the test's fixture operations.
The suite is registered in the isolated-stack Wekan E2E workflow matrix.
