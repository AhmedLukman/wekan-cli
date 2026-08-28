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

The ignored core-board lifecycle test requires a fresh isolated stack. Point it
at that stack and run only the dedicated test:

```powershell
$env:WEKAN_BOARD_E2E_URL = "http://localhost:3101"
cargo test --test e2e complete_board_lifecycle_matches_wekan_v11_06 -- --ignored --nocapture
```

Adding `--volumes` deletes only the selected project's database and uploaded
files. Omitting `--volumes` preserves them for the next start with that project
name.

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
