# Architecture

The CLI is a single Rust package with resource-based command, client, result,
and rendering modules. Its boundaries separate remote protocol behavior from
local orchestration and presentation.

## Request flow

1. `lib.rs` parses arguments and selects output behavior.
2. `App` executes local profile commands directly. For remote
   commands it validates command-level inputs, resolves one target, and retains
   that target's profile lease until dispatch finishes.
3. The resource family dispatches to a leaf handler. Authentication receives
   the mutable resolved target because login and registration can initialize a
   profile. Other families receive the client factory and only the external
   capabilities their handlers need.
4. The handler obtains credentials, applies confirmation policy where needed,
   invokes the client, and maps the response to a semantic command result.
5. `App` attaches profile context and any profile-initialization outcome to
   errors in one place.
6. The output layer writes a stable JSON envelope or formats the result for a
   human. Raw API bodies follow their separate streaming contract.

## Ownership

`App` owns root dispatch and passes explicit capabilities directly to resource
families or local handlers. This avoids a second prepared-command enum that
duplicates the root command vocabulary without adding a lifecycle boundary.
Family dispatch and operation behavior remain under `commands`; target leases
remain owned by `App` through the resolved target, and remote handlers create
their clients lazily.

| Module | Responsibility |
|---|---|
| `cli.rs`, `commands.rs` | Global arguments and the root command vocabulary |
| `app.rs` | Dependency composition, common target preparation, root dispatch, profile error context |
| `commands/<resource>.rs` | Resource vocabulary and leaf dispatch |
| `commands/<resource>/<operation>.rs` | Operation arguments, orchestration, and response-to-result mapping |
| `client/<resource>.rs` | Wekan v11.06 routes, request/response DTOs, and semantic validation |
| `client/transport.rs`, `client/decoding.rs` | Bounded HTTP execution, embedded Wekan errors, strict decoding, and safe diagnostics |
| `command_result/<resource>.rs` | Stable CLI result types, separate from wire DTOs |
| `output/<resource>.rs` | Human rendering for that resource |
| `output/formatting.rs` | Shared terminal escaping, tables, and nested-value formatting |
| `output.rs` | Format selection, shared envelopes, and output writes |
| `config/`, `credentials/`, `input/` | Profile storage and leases, native vault operations, secret input, and confirmation |

The result façade re-exports resource types and owns the exhaustive
`CommandSuccess` enum. The output façade chooses the renderer; resource modules
own presentation details. Adding a result requires updating the enum and its
format dispatch, while existing resource implementations stay in their own
files.

The HTTP client has no dependency on Clap command parsing or output rendering.
The CLI deliberately maps wire DTOs to result types so a protocol correction
does not implicitly change the public output contract. External side effects
use focused traits for credential storage, secret input, profile storage, and
confirmation. HTTP tests exercise the real client against a mock server.

## Command design

Commands use singular resources and verbs, such as `board get` and `card
update`. The resource's own ID is positional. Parent scope is explicit:

```console
wekan list get list-id --board board-id
wekan card get card-id --board board-id --list list-id
wekan comment get comment-id --board board-id --card card-id
wekan swimlane get swimlane-id --board board-id
```

These flags select the corresponding API route directly. They do not look up
names, infer parent IDs, or add requests. Help groups targeting, editable
fields, dates, people/labels, and global options.

## Verification

Unit tests cover policy, mapping, rendering, and failure paths. Black-box CLI
tests cover arguments, help, streams, and exit statuses. HTTP
contract tests cover actual request serialization and strict response decoding.
Live tests use isolated Wekan v11.06 stacks to verify server behavior.

See [the authentication architecture](auth-architecture.md) for transaction and
locking details, and [the agent contract](agent-contract.md) for output semantics.
