# Authentication architecture

This document describes the implemented `wekan auth register`,
`wekan auth login`, `wekan auth status`, and `wekan auth logout` slices. It
shows how command-line input becomes a Wekan request, how returned tokens cross
into and back out of the native credential store, and how success, failure, and
uncertain outcomes are reported. It also covers named-profile target resolution
and the lease that keeps profile configuration stable during authentication.

The behavioral details and stable machine contract remain authoritative in
[Authentication](auth.md) and [Agent output contract](agent-contract.md).

## System context

```mermaid
flowchart LR
    caller[Human or automation]

    subgraph cli[Wekan CLI process]
        entry[Argument parsing and output selection]
        app[Application composition]
        resolver[Server and profile target resolution]
        dispatch[Command dispatch]
        handlers[Registration, login, status, and logout handlers]
        secrets[Secret input provider]
        client[Wekan HTTP client]
        result[Result handling]
        output[Human or JSON renderer]
    end

    input[(Terminal or stdin)]
    wekan[(Wekan v11.06 REST API)]
    vault[(Native credential store)]
    profiles[(Versioned local profile store)]

    caller -->|arguments and environment| entry
    entry --> app
    app --> resolver
    resolver -->|shared existing or exclusive initialization lease| profiles
    resolver -->|resolved target: factory and lease| app
    app -->|prepared command and resolved target| dispatch
    dispatch --> handlers
    handlers --> secrets
    input -->|password and optional code| secrets
    handlers --> client
    client -->|Authentication POSTs or current-user GET| wekan
    handlers -->|preflight, load, save, or delete| vault
    app -->|CommandSuccess or AppError| result
    result --> output
    output -->|stdout on success; stderr on error| caller
```

The application layer owns concrete production dependencies, including the
long-lived `TargetResolver`. For an authentication command, `App` asks that
resolver for a command-scoped `ResolvedTarget`, retains it through dispatch,
and passes that focused target into root routing. The `ResolvedTarget` owns its
`WekanClientFactory`, required profile identity, and retained profile guard. The
`ProfileStore`, `CredentialStore`, and `SecretInputProvider` traits keep command
behavior testable without real local configuration, a terminal, or an
operating-system vault. The HTTP client owns URL and transport policy, shared
auth-session decoding, and allowlisted current-user decoding; each handler owns
operation-specific orchestration and error mapping.

## Component responsibilities

| Boundary | Responsibility | Key implementation |
| --- | --- | --- |
| Process entry | Detect requested output before parsing, parse the CLI, select stdout or stderr, and return a stable exit status | [`src/lib.rs`](../src/lib.rs), [`src/main.rs`](../src/main.rs) |
| CLI model | Define global selectors plus the profile, registration, login, status, and logout command shapes | [`src/cli.rs`](../src/cli.rs), [`src/commands.rs`](../src/commands.rs), [`src/commands/profile.rs`](../src/commands/profile.rs), [`src/commands/auth.rs`](../src/commands/auth.rs) |
| Application composition | Construct and own long-lived dependencies, decide which commands need a target, retain each resolved target through dispatch, and pass explicit capabilities into root routing | [`src/app.rs`](../src/app.rs) |
| Configuration boundary | Persist strict named profiles; own target-selection policy; canonicalize server identity independently of network permission; and produce a command-scoped target containing its client factory and retained shared or exclusive profile guard | [`src/config.rs`](../src/config.rs), [`src/config/profiles.rs`](../src/config/profiles.rs) |
| Authentication orchestration | Enforce operation order, load or store credentials, redact secrets, and map operation-specific errors | [`src/commands/auth/`](../src/commands/auth/) |
| Command result model | Define secret-free semantic success outcomes and shared outcome vocabulary independently of rendering | [`src/command_result.rs`](../src/command_result.rs) |
| HTTP boundary | Own the canonical URL type, enforce transport permission on client creation, and apply timeouts, no redirects, no retries, and loopback proxy bypass | [`src/client.rs`](../src/client.rs), [`src/client/auth.rs`](../src/client/auth.rs) |
| Secret boundary | Read confirmed registration passwords, single login passwords, and optional two-factor codes from non-echoing prompts or ordered stdin lines | [`src/credentials.rs`](../src/credentials.rs), [`src/credentials/`](../src/credentials/) |
| Output contract | Render terminal-escaped human success fields, human errors, or stable JSON envelopes without passwords or tokens | [`src/output.rs`](../src/output.rs), [`src/error.rs`](../src/error.rs), [`src/redaction.rs`](../src/redaction.rs) |

Dependencies point inward through traits where an external side effect needs a
test seam. Command modules may orchestrate the client and credential
abstractions, while the client and credential modules do not depend on command
parsing or output rendering.

## Target resolution and credential scope

Ordinary commands resolve the profile name through `--profile`,
`WEKAN_PROFILE`, the active profile, then `default`. They independently resolve
an optional URL through `--server`, then `WEKAN_URL`; both explicit selectors
may appear together. Profile-management commands reject explicit selectors and
ignore selector environment variables.

An existing profile supplies its stored canonical server URL. A supplied URL
must canonicalize to the same value or resolution returns
`profile_server_mismatch` before vault access. A missing profile normally
returns `profile_not_found`; login and registration may instead initialize it
when a URL was supplied, after a valid Wekan session has been decoded.
`auth logout --local-only` may instead remove its orphaned vault entry when a
URL is supplied, without recreating profile metadata.

The resolved target carries the expected canonical server URL and the
profile-only native-vault account key `profile:<store-identity>:<name>`. The
opaque store identity derives from the canonical profile configuration
directory, so independently overridden configuration stores cannot collide.
Older credential formats receive no compatibility lookup or mutation.

Resolving an existing authentication target retains a shared profile-store
lease through preflight, remote work, and final vault creation or deletion.
Missing-profile login and registration retain an exclusive mutation lease
through remote authentication, profile creation, and credential creation.
Every transaction therefore acquires profile locking before credential locking.

The configuration boundary's `TargetResolver` returns a `ResolvedTarget` that
owns the `WekanClientFactory`, required profile identity, and retained lease.
The resolver canonicalizes and validates server identity independently of
network permission, without initializing an HTTP client. `App` retains the
target for the complete command and passes it explicitly into root dispatch.
Root dispatch only routes. Auth-family dispatch gives login and registration
the mutable target needed to commit a pending profile through configuration-owned
behavior, while status and logout receive only its focused factory. Command code
does not resolve profiles or construct application infrastructure. The factory
enforces plaintext HTTP permission only when a remote handler calls `create()`;
local server-scoped operations use the canonical identity without a policy
bypass.

## Successful registration sequence

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant CLI as CLI entry
    participant App as Application
    participant Resolver as Target resolver
    participant Dispatch as Command dispatch
    participant Handler as Register handler
    participant Factory as Client factory
    participant Profiles as Profile store
    participant Vault as Native credential store
    participant Password as Password provider
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>CLI: auth register + identity + target selectors
    CLI->>CLI: Parse arguments and select output format
    CLI->>App: execute(RootCommand)
    App->>Resolver: Resolve profile and optional supplied server
    Resolver->>Resolver: Canonicalize identity (no network policy)
    Resolver-->>App: ResolvedTarget (factory + retained lease)
    Note over App,Factory: Factory exists; no HTTP client yet
    App->>Dispatch: dispatch(AuthCommand + ResolvedTarget)
    Dispatch->>Handler: execute(RegisterArgs + mutable target)
    Handler->>Factory: Create client
    Factory->>Factory: Enforce network transport policy
    Factory-->>Handler: Configured client + canonical URL
    Handler->>Vault: Acquire CLI mutation guards and check raw presence
    Vault-->>Handler: Available and absent
    Handler->>Password: Read password
    Password-->>Handler: SecretString
    Handler->>Client: register(identity, password)
    Client->>Wekan: POST users/register (JSON)
    Note over Client,Wekan: Connect timeout is 10 seconds
    Note over Client,Wekan: Total timeout is 30 seconds
    Note over Client,Wekan: Redirect following and automatic retries are disabled
    Wekan-->>Client: 200 {id, token, tokenExpires}
    Client->>Client: Enforce 1 MiB limit and validate fields
    Client-->>Handler: AuthSession
    Handler->>Profiles: Persist profile if it was missing
    Profiles-->>Handler: Created/active state
    Handler->>Vault: Check then create versioned record
    Note over Handler,Vault: Account key is profile-namespaced only
    Note over Handler,Vault: The password is never stored
    Vault-->>Handler: Saved
    Handler-->>Dispatch: AuthSuccess without token
    Dispatch-->>App: AuthSuccess
    App-->>CLI: AuthSuccess
    CLI-->>Caller: stdout + exit 0
```

The raw credential-presence check deliberately happens under mutation guards,
before password input and before the remote mutation. An entry present when the
CLI checks blocks authentication without being decoded or overwritten by that
CLI invocation. The guards serialize Wekan CLI mutations, not arbitrary native
vault clients: generic OS keyring APIs do not provide a cross-process atomic
create or compare-and-swap operation. An external writer can therefore race
between the check and the CLI's write. A successful response is not reported as
success until a missing profile has been persisted and the returned token has
been created in the vault. The token remains secret throughout: it is accepted
from Wekan, wrapped as secret data, written to the vault, and omitted from both
human and JSON output.

## Successful login sequence

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant Handler as Login handler
    participant Factory as Client factory
    participant Profiles as Profile store
    participant Vault as Native credential store
    participant Secrets as Secret input provider
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>Handler: auth login + one identity + resolved target
    Handler->>Factory: Create client and enforce network policy
    Factory-->>Handler: Configured client + canonical URL
    Handler->>Vault: Acquire CLI mutation guards; check raw entry absent
    Vault-->>Handler: Available and absent
    Handler->>Secrets: Read password and optional code
    Secrets-->>Handler: LoginSecrets
    Handler->>Client: login(identity, password, optional code)
    Client->>Wekan: One POST users/login (JSON)
    Note over Client,Wekan: Redirects and retries are disabled
    Wekan-->>Client: 200 {id, token, tokenExpires}
    Client-->>Handler: Validated AuthSession
    Handler->>Profiles: Persist profile if it was missing
    Profiles-->>Handler: Created/active state
    Handler->>Vault: Check then create credential
    Vault-->>Handler: Created
    Handler-->>Caller: Secret-free success envelope
```

Interactive `--code` and automated `--code-stdin` supply the code before this
single request. The CLI does not discover two-factor authentication by replaying
a rejected password-only request.

## Successful status sequence

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant Handler as Status handler
    participant Factory as Client factory
    participant Vault as Native credential store
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>Handler: auth status + resolved target
    Handler->>Factory: Create client and enforce network policy
    Factory-->>Handler: Configured client + canonical URL
    Handler->>Vault: Check availability and load record
    Vault-->>Handler: Valid version-1 credential
    Handler->>Handler: Reject expiry at or before now
    Handler->>Client: current_user(stored token)
    Client->>Wekan: GET api/user + bearer token
    Wekan-->>Client: 200 current-user data
    Client->>Client: Enforce limit and allowlist profile
    Client-->>Handler: CurrentUser
    Handler->>Handler: Require remote ID = stored ID
    Handler-->>Caller: Secret-free status envelope
```

A missing or expired record stops before HTTP. Wekan v11.06 serializes an
invalid-token error with `statusCode: 401` inside HTTP 200; the client preserves
both statuses and the handler returns `authentication_rejected`. Status never
saves, removes, or repairs a credential. An expired or rejected entry must be
removed with `auth logout --local-only` before login can create a replacement.

## Successful logout sequences

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant Handler as Logout handler
    participant Factory as Client factory
    participant Vault as Native credential store
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>Handler: auth logout [--all]
    Handler->>Factory: Create client and enforce network policy
    Handler->>Vault: Acquire CLI account/server mutation guards
    Handler->>Caller: Request confirmation unless --yes
    Caller-->>Handler: Approve
    Handler->>Vault: Preflight and load credential while guards are held
    Vault-->>Handler: Valid credential, including expired records
    Handler->>Client: logout(all, stored token)
    Client->>Wekan: POST users/logout + bearer token
    Wekan-->>Client: 200 {message}
    Client-->>Handler: Validated success
    Handler->>Vault: Compare loaded record, then delete if it still matches
    Vault-->>Handler: Deleted, absent, or observed replacement preserved
    Handler->>Vault: Release account mutation guard
    Handler-->>Caller: Secret-free logout scope
```

Remote errors stop before deletion. A malformed, unreadable, or oversized HTTP
200 records the observed status but sets `remote_logout_completed: null` and
`outcome_unknown: true`, then preserves the credential. A validated 200 followed
by deletion failure reports `remote_logout_completed: true` with
`credential_store_failed`.

The native credential store uses stable, private per-user application-data
lock paths. Login, registration, remote logout, and local-only logout acquire
both a target-account guard and a canonical-server guard before their remote
request and retain them until their local save or deletion completes. That
serializes the full authentication transaction across current Wekan CLI
processes and all profiles for a server, so an all-token logout cannot retain a
token that a concurrent CLI login had already created. Logout acquires both
guards before its confirmation prompt so approval cannot race with replacement
of the selected credential by another CLI mutation.

The locks do not make a generic native-vault write or conditional deletion
atomic. OS keyring APIs offer no cross-process atomic create or
compare-and-swap operation, so a non-cooperating external writer can race the
CLI between its presence or record comparison and its write or deletion.

For `auth logout --local-only`, target resolution supplies the expected server
URL and profile-namespaced vault account. With a supplied URL, it can also
target an otherwise missing profile solely to recover its orphaned vault entry;
that path retains a read guard and never recreates profile metadata. The handler
preflights and deletes the entry without creating an HTTP client, loading the
record, or contacting Wekan. Missing entries succeed idempotently with
`local_credential_removed: false`, and output warns that no remote tokens were
revoked.

## Registration validation and outcome flow

```mermaid
flowchart TD
    start([Command invoked]) --> parse{Arguments valid?}
    parse -- No --> usage[invalid_input<br/>exit 2]
    parse -- Yes --> target{Profile exists, or missing profile<br/>has a supplied valid URL?}
    target -- No --> config[profile_not_found, profile_server_mismatch,<br/>or configuration error; exit 3]
    target -- Yes --> server{Transport allowed?}
    server -- No --> insecure[insecure_transport<br/>exit 3]
    server -- Yes --> clientinit{HTTP client<br/>initialized?}
    clientinit -- No --> internal[internal_error<br/>exit 1]
    clientinit -- Yes --> vault{Credential store available<br/>and raw entry absent?}
    vault -- No --> unavailable[credential_store_unavailable<br/>exit 6<br/>account_created = false]
    vault -- Existing --> occupied[credential_already_exists<br/>exit 3<br/>no secrets or HTTP]
    vault -- Absent --> password{Password input valid?}
    password -- No --> input[invalid_input<br/>exit 2]
    password -- Yes --> endpoint{Registration endpoint<br/>constructed?}
    endpoint -- No --> endpointerr[protocol_error<br/>exit 4<br/>account_created = false]
    endpoint -- Yes --> request[Send one POST request]

    request --> response{Observed result}
    response -- Redirect --> redirect[unexpected_redirect<br/>exit 4]
    response -- Transport failure --> transport[transport_error<br/>exit 4<br/>outcome_unknown = true]
    response -- HTTP 400 --> rejected[registration_rejected<br/>exit 5<br/>outcome_unknown = true]
    response -- HTTP 403 --> disabled[registration_disabled<br/>exit 5]
    response -- HTTP 5xx --> servererr[server_error<br/>exit 5<br/>outcome_unknown = true]
    response -- Other non-success --> other[server_error<br/>exit 5]
    response -- HTTP 200 --> valid{Body within limit and<br/>session fields valid?}

    valid -- No --> protocol[protocol_error<br/>exit 4<br/>account_created = true]
    valid -- Yes --> expiry{Validated expiry<br/>formatted?}
    expiry -- No --> internal
    expiry -- Yes --> profilesave{Missing profile save outcome?}
    profilesave -- Not installed --> profilefail[configuration_error<br/>profile_created = false<br/>no credential write]
    profilesave -- Installed but not finalized --> profilefinalize[configuration_error<br/>profile_created = true<br/>no credential write]
    profilesave -- Saved --> save{Attempt credential<br/>creation}
    save -- Failed --> storefail[credential_store_failed<br/>exit 6<br/>account_created = true]
    save -- Existing --> late[credential_already_exists<br/>account_created = true<br/>preserve observed entry]
    save -- Created --> success([Success envelope<br/>profile_created/profile_active<br/>exit 0])
```

`account_created` states what the CLI knows about the mutation. An HTTP 200
with an unusable body still means Wekan reported success, and a vault write
failure occurs after a validated success. `outcome_unknown` instead marks cases
where the request may have taken effect but the CLI cannot prove it. This is
especially important because registration is intentionally never retried.

## Login validation and outcome flow

```mermaid
flowchart TD
    start([Login invoked]) --> parse{Exactly one identity and<br/>valid secret-input mode?}
    parse -- No --> usage[invalid_input<br/>exit 2]
    parse -- Yes --> server{Profile target and client valid?}
    server -- No --> config[profile/configuration/internal error]
    server -- Yes --> vault{Credential store available<br/>and raw entry absent?}
    vault -- No --> unavailable[credential_store_unavailable<br/>session_created = false]
    vault -- Existing --> occupied[credential_already_exists<br/>session_created = false<br/>no secrets or HTTP]
    vault -- Absent --> secrets{Password and optional code valid?}
    secrets -- No --> input[invalid_input<br/>exit 2]
    secrets -- Yes --> request[Send one POST users/login]
    request --> response{Observed result}
    response -- Transport or 5xx --> unknown[outcome_unknown = true]
    response -- 400 --> malformed[protocol_error<br/>exit 4]
    response -- 401 --> rejected[login_rejected<br/>exit 5]
    rejected --> twofactor{no-2fa-code?}
    twofactor -- Yes --> required[two_factor_required = true<br/>retry explicitly with code]
    twofactor -- No --> credentials[credentials or code rejected]
    response -- 429 --> limited[login_rate_limited<br/>retry_after_seconds]
    response -- Other error --> servererr[server_error<br/>exit 5]
    response -- 200 unusable --> protocol[protocol_error<br/>session_created = true]
    response -- 200 valid --> profilesave{Missing profile save outcome?}
    profilesave -- Not installed --> profilefail[configuration_error<br/>session_created = true<br/>profile_created = false<br/>no credential write]
    profilesave -- Installed but not finalized --> profilefinalize[configuration_error<br/>session_created = true<br/>profile_created = true<br/>no credential write]
    profilesave -- Saved --> save{Attempt credential<br/>creation}
    save -- Failed --> storefail[credential_store_failed<br/>session_created = true]
    save -- Existing --> late[credential_already_exists<br/>session_created = true<br/>preserve observed entry]
    save -- Created --> success([Success envelope<br/>profile_created/profile_active<br/>exit 0])
```

`session_created` is the login counterpart to `account_created`. Authentication
does not replace an entry it observes during its guarded CLI transaction; a
caller must explicitly log out the profile before requesting another session.

## Status validation flow

```mermaid
flowchart TD
    start([Status invoked]) --> server{Server and client valid?}
    server -- No --> config[configuration/internal error]
    server -- Yes --> vault{Credential store available?}
    vault -- No --> unavailable[credential_store_unavailable<br/>exit 6]
    vault -- Yes --> load{Record loads and validates?}
    load -- Missing --> missing[credential_not_found<br/>exit 5]
    load -- Invalid --> corrupt[credential_store_failed<br/>exit 6]
    load -- Valid --> expiry{Expiry in future?}
    expiry -- No --> expired[credential_expired<br/>exit 5<br/>no HTTP]
    expiry -- Yes --> request[GET api/user]
    request --> response{Observed result}
    response -- Transport/redirect --> transport[transport error<br/>exit 4]
    response -- HTTP 200 + embedded 401 --> rejected[authentication_rejected<br/>exit 5]
    response -- Other Wekan error --> servererr[server_error<br/>exit 5]
    response -- Malformed/oversized --> protocol[protocol_error<br/>exit 4]
    response -- Valid user --> match{Remote ID matches stored ID?}
    match -- No --> mismatch[credential_store_failed<br/>exit 6]
    match -- Yes --> success([Allowlisted status envelope<br/>exit 0])
```

## Logout validation flow

```mermaid
flowchart TD
    start([Logout invoked]) --> mode{Local only?}
    mode -- Yes --> key[Resolve target account key]
    key --> localdelete{Delete vault entry}
    localdelete -- Deleted or absent --> localsuccess([Local-only success<br/>no HTTP])
    localdelete -- Failed --> localfail[credential_store_failed<br/>exit 6]
    mode -- No --> client[Create client and canonical URL]
    client --> load{Credential loads?}
    load -- Missing --> missing[credential_not_found<br/>exit 5]
    load -- Invalid --> invalid[credential_store_failed<br/>exit 6]
    load -- Valid or expired --> request[POST users/logout]
    request --> response{Observed result}
    response -- Transport or 5xx --> unknown[Preserve credential<br/>outcome_unknown = true]
    response -- 401 --> rejected[authentication_rejected<br/>preserve credential]
    response -- Redirect/other error --> error[Preserve credential<br/>mapped error]
    response -- Unusable 200 --> protocol[protocol_error<br/>remote_logout_completed = null<br/>outcome_unknown = true<br/>preserve credential]
    response -- Valid 200 --> delete{Stored record still matches?}
    delete -- Failed --> deletefail[credential_store_failed<br/>remote_logout_completed = true]
    delete -- Yes --> removed[Delete matching record<br/>local_credential_removed = true]
    delete -- Replaced --> preserve[Preserve newer record<br/>credential_stored = true]
    delete -- Absent --> absent[Already absent<br/>local_credential_removed = false]
    removed --> success([Logout success<br/>exit 0])
    preserve --> success
    absent --> success
```

## Data and trust boundaries

```mermaid
flowchart LR
    subgraph untrusted[Untrusted or externally controlled data]
        args[Arguments and environment]
        response[HTTP status and response body]
        preflight[Credential preflight errors]
        storeerr[Credential load, save, or delete errors]
    end

    subgraph controls[CLI controls]
        validation[CLI and URL validation]
        limits[Timeouts, no automatic mutation retries,<br/>redirect policy, 1 MiB response limit]
        redaction[Known-secret redaction]
        successmeta[Validated success metadata]
        escaping[Human success-field<br/>terminal escaping]
        rendered[Human output or<br/>stable JSON envelope]
    end

    subgraph secrets[Secret-bearing path]
        secretinput[Non-echoing prompt or ordered stdin]
        pass[Password]
        code[Optional two-factor code]
        token[Returned or loaded login token]
        vault[(Native credential store)]
    end

    args --> validation --> limits
    secretinput --> pass
    secretinput --> code
    pass -->|request body only| limits
    code -->|login request body only| limits
    response --> limits
    limits --> token
    token <--> vault
    limits --> redaction
    limits --> successmeta
    successmeta --> escaping
    preflight --> rendered
    storeerr --> redaction
    redaction --> rendered
    escaping --> rendered

    pass -. never persisted .-> redaction
    code -. never persisted .-> redaction
    token -. never rendered .-> rendered
```

HTTP response bodies are bounded and classified before output. When a password,
two-factor code, or token could be echoed by a remote or credential-store error,
that known secret is replaced. Terminal control characters are escaped in human success
fields, and structured JSON exposes only documented metadata. Human error
rendering does not apply a general terminal-control escaping pass.

## Verification layers

```mermaid
flowchart BT
    unit[Unit tests<br/>profile storage and leases, validation,<br/>mapping, redaction, and credential records]
    contract[HTTP contract tests<br/>request shape, response decoding,<br/>limits, redirects and no retries]
    cli[Black-box CLI tests<br/>profile commands, selectors, streams,<br/>JSON errors and stable exit statuses]
    e2e[Ignored live Wekan test<br/>authentication lifecycle, local cleanup,<br/>current-token and all-token revocation]

    unit --> contract --> cli --> e2e
```
