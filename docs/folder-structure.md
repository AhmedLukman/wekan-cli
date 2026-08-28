# Repository folder structure

## Target layout

```text
wekan-cli/
├── Cargo.toml                         # Package metadata, dependencies, features and lint rules
├── Cargo.lock                         # Locked dependencies; committed for this CLI application
├── rust-toolchain.toml                # Tested Rust toolchain plus rustfmt and Clippy
├── deny.toml                          # Dependency, advisory, source and licence policy
├── dist-workspace.toml                # Cross-platform cargo-dist release configuration
│
├── README.md                          # Installation, quick start and command overview
├── CHANGELOG.md                       # Release history
├── CONTRIBUTING.md                    # Contribution and development instructions
├── SECURITY.md                        # Vulnerability reporting policy
├── LICENSE
├── .gitignore
├── .editorconfig
│
├── src/
│   ├── main.rs                        # Minimal executable entry point and async runtime
│   ├── lib.rs                         # Testable application entry point and module declarations
│   ├── app.rs                         # Composition root; owns target resolver and shared dependencies
│   ├── cli.rs                         # Root Cli parser and global flags only
│   ├── command_result.rs              # Stable semantic command outcomes shared by commands, errors and output
│   ├── error.rs                       # Application-level error representation
│   ├── exit_code.rs                   # Stable documented process exit codes
│   ├── redaction.rs                   # Central secret-redaction policy
│   │
│   ├── commands.rs                    # Root command enum, module declarations and dispatch
│   ├── commands/
│   │   ├── authenticated.rs           # Handler-owned authenticated command state
│   │   ├── client_error.rs            # Shared client-to-command error-detail mapping
│   │   ├── credential_ops.rs          # Shared credential-store operation policy
│   │   │
│   │   ├── profile.rs                 # `wekan profile` family enum, dispatch and shared policy
│   │   ├── profile/
│   │   │   ├── add.rs                 # AddArgs and add handler
│   │   │   ├── list.rs                # ListArgs and list handler
│   │   │   ├── show.rs                # ShowArgs and show handler
│   │   │   ├── use_profile.rs         # UseArgs and active-profile selection handler
│   │   │   ├── update.rs              # UpdateArgs and update handler
│   │   │   ├── remove.rs              # RemoveArgs and remove handler
│   │   │   └── tests.rs               # Profile-family unit tests and shared fixtures
│   │   │
│   │   ├── auth.rs                    # `wekan auth` family enum and dispatch
│   │   ├── auth/
│   │   │   ├── login.rs               # LoginArgs and login handler
│   │   │   ├── logout.rs              # LogoutArgs and logout handler
│   │   │   ├── status.rs              # Authentication/profile status handler
│   │   │   └── ...
│   │   │
│   │   ├── config.rs                  # `wekan config` family enum and dispatch
│   │   ├── config/
│   │   │   ├── get.rs                 # Read a configuration value
│   │   │   ├── set.rs                 # Write a configuration value
│   │   │   └── ...
│   │   │
│   │   ├── users.rs                   # `wekan user` family
│   │   ├── users/
│   │   │   ├── current.rs
│   │   │   ├── cards.rs
│   │   │   ├── list.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   ├── boards.rs
│   │   │   ├── take_ownership.rs
│   │   │   ├── disable_login.rs
│   │   │   ├── enable_login.rs
│   │   │   ├── delete.rs
│   │   │   └── tests.rs
│   │   │
│   │   ├── boards.rs                  # `wekan board` family
│   │   ├── boards/
│   │   │   ├── list.rs
│   │   │   ├── count.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   ├── rename.rs
│   │   │   ├── delete.rs
│   │   │   └── tests.rs
│   │   │
│   │   ├── lists.rs                   # `wekan list` family
│   │   ├── lists/
│   │   │   ├── list.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── swimlanes.rs               # `wekan swimlane` family
│   │   ├── swimlanes/
│   │   │   ├── list.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── cards.rs                   # `wekan card` family
│   │   ├── cards/
│   │   │   ├── list.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   ├── move_card.rs           # Exposes `wekan card move`
│   │   │   └── ...
│   │   │
│   │   ├── comments.rs                # `wekan comment` family
│   │   ├── comments/
│   │   │   ├── list.rs
│   │   │   ├── get.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── checklists.rs              # `wekan checklist` family
│   │   ├── checklists/
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   ├── add_item.rs
│   │   │   └── ...
│   │   │
│   │   ├── custom_fields.rs           # `wekan custom-field` family
│   │   ├── custom_fields/
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── attachments.rs             # `wekan attachment` family
│   │   ├── attachments/
│   │   │   ├── list.rs
│   │   │   ├── upload.rs
│   │   │   ├── download.rs
│   │   │   └── ...
│   │   │
│   │   ├── integrations.rs            # `wekan integration` family
│   │   ├── integrations/
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── rules.rs                   # `wekan rule` family
│   │   ├── rules/
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── dependencies.rs            # `wekan dependency` family
│   │   ├── dependencies/
│   │   │   ├── list.rs
│   │   │   ├── create.rs
│   │   │   └── ...
│   │   │
│   │   ├── admin.rs                   # `wekan admin` family
│   │   ├── admin/
│   │   │   ├── stats.rs
│   │   │   ├── settings.rs
│   │   │   └── ...
│   │   │
│   │   ├── api.rs                     # User-facing `wekan api` command family
│   │   ├── api/
│   │   │   └── request.rs             # Raw same-origin REST request command
│   │   │
│   │   ├── completion.rs              # Generates shell completions
│   │   ├── doctor.rs                  # Configuration, connectivity and auth diagnostics
│   │   └── ...
│   │
│   ├── client.rs                      # WekanClient façade and resource re-exports
│   ├── client/
│   │   ├── error.rs                   # HTTP, decoding and Wekan response errors
│   │   ├── ids.rs                     # BoardId, CardId, UserId and other typed IDs
│   │   ├── datetime.rs                # Wekan date/time parsing and serialization
│   │   │
│   │   ├── transport.rs               # Shared HTTP request-execution façade
│   │   ├── transport/
│   │   │   ├── authorization.rs       # Secure authorization-header behavior
│   │   │   ├── retry.rs               # Retry classification and backoff
│   │   │   ├── redirect.rs            # Redirect and credential-leak protection
│   │   │   └── ...
│   │   │
│   │   ├── auth.rs                    # Authentication endpoints and DTOs
│   │   │
│   │   ├── users.rs                   # Core user endpoints and DTOs
│   │   ├── users/
│   │   │   ├── admin.rs
│   │   │   ├── settings.rs
│   │   │   └── ...
│   │   │
│   │   ├── boards.rs                  # Core board endpoints and DTOs
│   │   ├── boards/
│   │   │   ├── members.rs
│   │   │   ├── labels.rs
│   │   │   ├── import_export.rs
│   │   │   └── ...
│   │   │
│   │   ├── lists.rs                   # List endpoints and DTOs
│   │   ├── lists/
│   │   │   ├── ordering.rs
│   │   │   └── ...
│   │   │
│   │   ├── swimlanes.rs               # Swimlane endpoints and DTOs
│   │   ├── swimlanes/
│   │   │   ├── ordering.rs
│   │   │   └── ...
│   │   │
│   │   ├── cards.rs                   # Core card endpoints and DTOs
│   │   ├── cards/
│   │   │   ├── movement.rs
│   │   │   ├── assignees.rs
│   │   │   ├── custom_fields.rs
│   │   │   └── ...
│   │   │
│   │   ├── comments.rs                # Comment endpoints and DTOs
│   │   ├── checklists.rs              # Checklist endpoints and DTOs
│   │   ├── checklists/
│   │   │   ├── items.rs
│   │   │   └── ...
│   │   │
│   │   ├── custom_fields.rs           # Custom-field definition endpoints and DTOs
│   │   ├── attachments.rs             # Attachment endpoints and DTOs
│   │   ├── attachments/
│   │   │   ├── upload.rs
│   │   │   ├── download.rs
│   │   │   └── ...
│   │   │
│   │   ├── integrations.rs
│   │   ├── rules.rs
│   │   ├── dependencies.rs
│   │   ├── admin.rs
│   │   └── ...
│   │
│   ├── credentials.rs                 # Credential abstraction and provider selection
│   ├── credentials/
│   │   ├── keyring.rs                 # Windows, macOS and Linux credential stores
│   │   ├── environment.rs             # Credentials provided through environment variables
│   │   ├── stdin.rs                   # Piped secret input
│   │   ├── prompt.rs                  # Non-echoing interactive secret prompt
│   │   └── ...
│   │
│   ├── workflows.rs                   # Genuine multi-call workflow declarations
│   ├── workflows/
│   │   ├── migration.rs               # Cross-instance export, mapping and import
│   │   ├── ensure.rs                  # Idempotent find-or-create behavior
│   │   ├── resolve.rs                 # Resolve names/slugs into Wekan IDs
│   │   └── ...
│   │
│   ├── config.rs                      # Configuration façade, target resolver, and shared types
│   ├── config/
│   │   ├── load.rs                    # Configuration loading and precedence
│   │   ├── store.rs                   # Configuration persistence
│   │   ├── environment.rs             # Non-secret environment configuration
│   │   ├── profiles.rs                # Named Wekan server profiles
│   │   └── ...
│   │
│   ├── input.rs                       # Shared non-credential input façade
│   ├── input/
│   │   ├── payload.rs                 # JSON/YAML from arguments, files or stdin
│   │   ├── confirmation.rs            # Interactive confirmation and `--yes`
│   │   └── ...
│   │
│   ├── output.rs                      # Output selection and rendering façade
│   └── output/
│       ├── envelope.rs                # Stable machine-readable success/error envelope
│       ├── filter.rs                  # Structured field selection and filtering
│       ├── json.rs                    # JSON renderer
│       ├── jsonl.rs                   # Streaming JSON Lines renderer
│       ├── table.rs                   # Human-readable table renderer
│       ├── yaml.rs                    # YAML renderer
│       ├── raw.rs                     # Unformatted response/body output
│       └── ...
│
├── spec/
│   ├── wekan.yml                      # Untouched upstream OpenAPI specification
│   ├── wekan.overlay.yml              # Maintained corrections and additions
│   ├── wekan.corrected.yml            # Generated merged specification
│   ├── wekan.html                     # Rendered upstream API documentation
│   └── wekan.corrected.html           # Rendered corrected API documentation
│
├── tests/
│   ├── cli.rs                         # Black-box CLI integration-test target
│   ├── cli/
│   │   ├── auth.rs                    # Authentication command tests
│   │   ├── boards.rs                  # Board command tests
│   │   ├── cards.rs                   # Card command tests
│   │   ├── api.rs                     # Raw API command tests
│   │   ├── output.rs                  # Machine-output contract tests
│   │   └── ...
│   │
│   ├── contract.rs                    # API contract-test target
│   ├── contract/
│   │   ├── coverage.rs                # Endpoint-to-command coverage checks
│   │   ├── requests.rs                # Serialized request checks
│   │   ├── responses.rs               # Response compatibility checks
│   │   └── ...
│   │
│   ├── e2e.rs                         # Live/containerized Wekan test target
│   ├── e2e/
│   │   ├── auth.rs                    # Authentication lifecycle tests
│   │   ├── boards.rs                  # Board lifecycle tests
│   │   ├── cards.rs                   # Card lifecycle tests
│   │   └── ...
│   │
│   ├── support/
│   │   ├── mod.rs                     # Shared integration-test infrastructure
│   │   ├── server.rs                  # Test Wekan server/container management
│   │   ├── assertions.rs              # Reusable CLI and JSON assertions
│   │   └── ...
│   │
│   ├── fixtures/
│   │   ├── requests/
│   │   │   ├── create_board.json
│   │   │   └── ...
│   │   └── responses/
│   │       ├── board.json
│   │       └── ...
│   │
│   └── snapshots/
│       ├── boards_list.snap
│       └── ...
│
├── docs/
│   ├── architecture.md                # Architectural boundaries and dependency rules
│   ├── agent-contract.md              # JSON contracts, stdout/stderr and exit codes
│   ├── auth.md                        # Authentication and credential behavior
│   ├── boards.md                      # Core board lifecycle and stable results
│   ├── config.md                      # Profiles, environment and precedence
│   ├── compatibility.md               # Supported Wekan versions
│   ├── development.md                 # Local development and testing
│   ├── examples.md                    # Human and agent automation examples
│   ├── adr/
│   │   ├── 0001-client-boundary.md
│   │   ├── 0002-command-ownership.md
│   │   ├── 0003-credentials.md
│   │   └── ...
│   └── ...
│
└── .github/
    ├── workflows/
    │   ├── ci.yml                     # Formatting, Clippy and automated tests
    │   ├── e2e.yml                    # Cross-platform Wekan integration tests
    │   ├── api-drift.yml              # Upstream API drift detection
    │   ├── codeql.yml                 # Security static analysis
    │   ├── dependency-review.yml      # Pull-request dependency review
    │   ├── release.yml                # Cross-platform release publication
    │   └── ...
    │
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.yml
    │   ├── feature_request.yml
    │   ├── config.yml
    │   └── ...
    │
    ├── dependabot.yml
    └── pull_request_template.md

```
