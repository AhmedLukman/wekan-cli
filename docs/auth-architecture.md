# Authentication architecture

This document describes the implemented `wekan auth register`,
`wekan auth login`, `wekan auth status`, and `wekan auth logout` slices. It
shows how command-line input becomes a Wekan request, how returned tokens cross
into and back out of the native credential store, and how success, failure, and
uncertain outcomes are reported.

The behavioral details and stable machine contract remain authoritative in
[Authentication](auth.md) and [Agent output contract](agent-contract.md).

## System context

```mermaid
flowchart LR
    caller[Human or automation]

    subgraph cli[Wekan CLI process]
        entry[Argument parsing and output selection]
        app[Application composition]
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

    caller -->|arguments and environment| entry
    entry --> app
    app --> dispatch
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

The application layer owns concrete production dependencies and injects them
into the command handler. The `CredentialStore` and `SecretInputProvider` traits
keep command behavior testable without a real terminal or operating-system
vault. The HTTP client owns URL, transport policy, shared auth-session decoding,
and allowlisted current-user decoding; each handler owns operation-specific
orchestration and error mapping.

## Component responsibilities

| Boundary | Responsibility | Key implementation |
| --- | --- | --- |
| Process entry | Detect requested output before parsing, parse the CLI, select stdout or stderr, and return a stable exit status | [`src/lib.rs`](../src/lib.rs), [`src/main.rs`](../src/main.rs) |
| CLI model | Define global flags plus the registration, login, status, and logout command shapes | [`src/cli.rs`](../src/cli.rs), [`src/commands.rs`](../src/commands.rs), [`src/commands/auth.rs`](../src/commands/auth.rs) |
| Application composition | Construct production dependencies and pass them into dispatch | [`src/app.rs`](../src/app.rs) |
| Authentication orchestration | Enforce operation order, load or store credentials, redact secrets, and map operation-specific errors | [`src/commands/auth/`](../src/commands/auth/) |
| Command result model | Define secret-free semantic success outcomes and shared outcome vocabulary independently of rendering | [`src/command_result.rs`](../src/command_result.rs) |
| HTTP boundary | Canonicalize and validate the server URL; apply timeouts, no redirects, no retries, and loopback proxy bypass | [`src/client.rs`](../src/client.rs), [`src/client/auth.rs`](../src/client/auth.rs) |
| Secret boundary | Read confirmed registration passwords, single login passwords, and optional two-factor codes from non-echoing prompts or ordered stdin lines | [`src/credentials.rs`](../src/credentials.rs), [`src/credentials/`](../src/credentials/) |
| Output contract | Render terminal-escaped human success fields, human errors, or stable JSON envelopes without passwords or tokens | [`src/output.rs`](../src/output.rs), [`src/error.rs`](../src/error.rs), [`src/redaction.rs`](../src/redaction.rs) |

Dependencies point inward through traits where an external side effect needs a
test seam. Command modules may orchestrate the client and credential
abstractions, while the client and credential modules do not depend on command
parsing or output rendering.

## Successful registration sequence

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant CLI as CLI entry
    participant App as Application
    participant Dispatch as Command dispatch
    participant Handler as Register handler
    participant Factory as Client factory
    participant Vault as Native credential store
    participant Password as Password provider
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>CLI: auth register + identity + server
    CLI->>CLI: Parse arguments and select output format
    CLI->>App: execute(RootCommand)
    App->>Dispatch: dispatch(AuthCommand)
    Dispatch->>Handler: execute(RegisterArgs)
    Handler->>Factory: Create client
    Factory->>Factory: Validate and canonicalize server URL
    Factory-->>Handler: Configured client + canonical URL
    Handler->>Vault: Check availability for canonical URL
    Vault-->>Handler: Available
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
    Handler->>Vault: Save versioned credential record
    Note over Handler,Vault: Account key is the canonical server URL
    Note over Handler,Vault: The password is never stored
    Vault-->>Handler: Saved
    Handler-->>Dispatch: AuthSuccess without token
    Dispatch-->>App: AuthSuccess
    App-->>CLI: AuthSuccess
    CLI-->>Caller: stdout + exit 0
```

The credential-store preflight deliberately happens before password input and
before the remote mutation. A successful response is not reported as success
until the returned token has been stored. The token remains secret throughout:
it is accepted from Wekan, wrapped as secret data, written to the vault, and
omitted from both human and JSON output.

## Successful login sequence

```mermaid
sequenceDiagram
    autonumber
    actor Caller as Human or agent
    participant Handler as Login handler
    participant Factory as Client factory
    participant Vault as Native credential store
    participant Secrets as Secret input provider
    participant Client as Wekan client
    participant Wekan as Wekan v11.06

    Caller->>Handler: auth login + one identity + server
    Handler->>Factory: Create and canonicalize client
    Factory-->>Handler: Configured client + canonical URL
    Handler->>Vault: Check availability
    Vault-->>Handler: Available
    Handler->>Secrets: Read password and optional code
    Secrets-->>Handler: LoginSecrets
    Handler->>Client: login(identity, password, optional code)
    Client->>Wekan: One POST users/login (JSON)
    Note over Client,Wekan: Redirects and retries are disabled
    Wekan-->>Client: 200 {id, token, tokenExpires}
    Client-->>Handler: Validated AuthSession
    Handler->>Vault: Replace credential for canonical URL
    Vault-->>Handler: Saved
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

    Caller->>Handler: auth status + server
    Handler->>Factory: Create and canonicalize client
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
saves, removes, or repairs a credential.

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
    Handler->>Factory: Create and canonicalize client
    Handler->>Vault: Preflight and acquire account mutation guard
    Handler->>Vault: Load credential while the guard is held
    Vault-->>Handler: Valid credential, including expired records
    Handler->>Client: logout(all, stored token)
    Client->>Wekan: POST users/logout + bearer token
    Wekan-->>Client: 200 {message}
    Client-->>Handler: Validated success
    Handler->>Vault: Delete only if record still matches
    Vault-->>Handler: Deleted, absent, or replacement preserved
    Handler->>Vault: Release account mutation guard
    Handler-->>Caller: Secret-free logout scope
```

Remote errors stop before deletion. A malformed, unreadable, or oversized HTTP
200 records the observed status but sets `remote_logout_completed: null` and
`outcome_unknown: true`, then preserves the credential. A validated 200 followed
by deletion failure reports `remote_logout_completed: true` with
`credential_store_failed`.

The native credential store uses a stable, private per-user application-data
lock path. Login, registration, remote logout, and local-only logout acquire
the same canonical-server guard before their remote request and retain it until
their local save or deletion completes. That serializes the full authentication
transaction across current CLI processes, so an all-token logout cannot retain
a token that a concurrent login had already created.

For `auth logout --local-only`, the factory only canonicalizes the configured
server URL. The handler preflights and deletes the vault entry without creating
an HTTP client, loading the record, or contacting Wekan. Missing entries succeed
idempotently with `local_credential_removed: false`, and output warns that no
remote tokens were revoked.

## Registration validation and outcome flow

```mermaid
flowchart TD
    start([Command invoked]) --> parse{Arguments valid?}
    parse -- No --> usage[invalid_input<br/>exit 2]
    parse -- Yes --> server{Server present, valid,<br/>and transport allowed?}
    server -- No --> config[configuration_error or insecure_transport<br/>exit 3]
    server -- Yes --> clientinit{HTTP client<br/>initialized?}
    clientinit -- No --> internal[internal_error<br/>exit 1]
    clientinit -- Yes --> vault{Credential store<br/>available?}
    vault -- No --> unavailable[credential_store_unavailable<br/>exit 6<br/>account_created = false]
    vault -- Yes --> password{Password input valid?}
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
    expiry -- Yes --> save{Credential saved?}
    save -- No --> storefail[credential_store_failed<br/>exit 6<br/>account_created = true]
    save -- Yes --> success([Success envelope<br/>exit 0])
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
    parse -- Yes --> server{Server and client valid?}
    server -- No --> config[configuration/internal error]
    server -- Yes --> vault{Credential store available?}
    vault -- No --> unavailable[credential_store_unavailable<br/>session_created = false]
    vault -- Yes --> secrets{Password and optional code valid?}
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
    response -- 200 valid --> save{Credential saved?}
    save -- No --> storefail[credential_store_failed<br/>session_created = true]
    save -- Yes --> success([Success envelope<br/>exit 0])
```

`session_created` is the login counterpart to `account_created`. Local
credential replacement does not revoke any older tokens that Wekan has issued.

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
    mode -- Yes --> key[Canonicalize server key]
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
    unit[Unit tests<br/>validation, mapping, redaction,<br/>password and credential records]
    contract[HTTP contract tests<br/>request shape, response decoding,<br/>limits, redirects and no retries]
    cli[Black-box CLI tests<br/>streams, JSON errors, precedence<br/>and stable exit statuses]
    e2e[Ignored live Wekan test<br/>authentication lifecycle, local cleanup,<br/>current-token and all-token revocation]

    unit --> contract --> cli --> e2e
```
