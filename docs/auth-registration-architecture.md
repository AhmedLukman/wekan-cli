# Authentication registration architecture

This document describes the implemented `wekan auth register` slice. It shows
how command-line input becomes one non-idempotent Wekan request, how the
returned login token crosses the process boundary into the native credential
store, and how success, failure, and uncertain outcomes are reported.

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
        register[Registration handler]
        password[Password provider]
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
    dispatch --> register
    register --> password
    input -->|password, never an argument| password
    register --> client
    client -->|POST users/register| wekan
    register -->|preflight and save| vault
    app -->|CommandSuccess or AppError| result
    result --> output
    output -->|stdout on success; stderr on error| caller
```

The application layer owns concrete production dependencies and injects them
into the command handler. The `CredentialStore` and `PasswordProvider` traits
keep command behavior testable without a real terminal or operating-system
vault. The HTTP client owns URL and transport policy; the registration handler
owns orchestration, error mapping, and mutation-state metadata.

## Component responsibilities

| Boundary | Responsibility | Key implementation |
| --- | --- | --- |
| Process entry | Detect requested output before parsing, parse the CLI, select stdout or stderr, and return a stable exit status | [`src/lib.rs`](../src/lib.rs), [`src/main.rs`](../src/main.rs) |
| CLI model | Define global server, output, and insecure-HTTP flags plus the `auth register` command shape | [`src/cli.rs`](../src/cli.rs), [`src/commands.rs`](../src/commands.rs), [`src/commands/auth.rs`](../src/commands/auth.rs) |
| Application composition | Construct production dependencies and pass them into dispatch | [`src/app.rs`](../src/app.rs) |
| Registration orchestration | Enforce operation order, construct the request, store credentials, redact secrets, and map errors | [`src/commands/auth/register.rs`](../src/commands/auth/register.rs) |
| HTTP boundary | Canonicalize and validate the server URL; apply timeouts, no redirects, no retries, and loopback proxy bypass | [`src/client.rs`](../src/client.rs), [`src/client/auth.rs`](../src/client/auth.rs) |
| Secret boundary | Read a password from a non-echoing prompt or one stdin line and persist only the returned token in the native vault | [`src/credentials.rs`](../src/credentials.rs), [`src/credentials/`](../src/credentials/) |
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
    Handler-->>Dispatch: RegistrationSuccess without token
    Dispatch-->>App: RegistrationSuccess
    App-->>CLI: RegistrationSuccess
    CLI-->>Caller: stdout + exit 0
```

The credential-store preflight deliberately happens before password input and
before the remote mutation. A successful response is not reported as success
until the returned token has been stored. The token remains secret throughout:
it is accepted from Wekan, wrapped as secret data, written to the vault, and
omitted from both human and JSON output.

## Validation and outcome flow

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

## Data and trust boundaries

```mermaid
flowchart LR
    subgraph untrusted[Untrusted or externally controlled data]
        args[Arguments and environment]
        response[HTTP status and response body]
        preflight[Credential preflight errors]
        storeerr[Credential save errors]
    end

    subgraph controls[CLI controls]
        validation[CLI and URL validation]
        limits[Timeouts, redirect and retry policy,<br/>1 MiB response limit]
        redaction[Known-secret redaction]
        successmeta[Validated success metadata]
        escaping[Human success-field<br/>terminal escaping]
        rendered[Human output or<br/>stable JSON envelope]
    end

    subgraph secrets[Secret-bearing path]
        secretinput[Password prompt or stdin]
        pass[Registration password]
        token[Returned login token]
        vault[(Native credential store)]
    end

    args --> validation --> limits
    secretinput --> pass
    pass -->|request body only| limits
    response --> limits
    limits --> token
    token --> vault
    limits --> redaction
    limits --> successmeta
    successmeta --> escaping
    preflight --> rendered
    storeerr --> redaction
    redaction --> rendered
    escaping --> rendered

    pass -. never persisted .-> redaction
    token -. never rendered .-> rendered
```

HTTP response bodies are bounded and classified before output. When a password
or token could be echoed by a remote or credential-store error, that known
secret is replaced. Terminal control characters are escaped in human success
fields, and structured JSON exposes only documented metadata. Human error
rendering does not apply a general terminal-control escaping pass.

## Verification layers

```mermaid
flowchart BT
    unit[Unit tests<br/>validation, mapping, redaction,<br/>password and credential records]
    contract[HTTP contract tests<br/>request shape, response decoding,<br/>limits, redirects and no retries]
    cli[Black-box CLI tests<br/>streams, JSON errors, precedence<br/>and stable exit statuses]
    e2e[Ignored live Wekan test<br/>create user, reject duplicate,<br/>authenticate with stored token]

    unit --> contract --> cli --> e2e
```
