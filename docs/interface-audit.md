# Wekan v12.57 interface and route audit

The supported baseline is Wekan v12.57, commit
[`647cbd111f43b4c936af2e03480f04cb8e2ef39e`](https://github.com/wekan/wekan/tree/v12.57).
The package version remains **1.0.0**. The parser has **166 leaf commands in
18 root families**, mapping **167 of 168 audited native REST operations**.
The Amazon SES/SNS operation is intentionally excluded from typed CLI support.
[API coverage](api-coverage.md) lists every argument, optional/global flag,
repeated input, conflict and endpoint selector. The
[command inventory](command-contract.md) covers local commands as well.

## Route discovery and boundaries

The audit reviewed 194 literal route registrations, dynamic registrations and
modules imported by
[`server/imports.js`](https://github.com/wekan/wekan/blob/v12.57/server/imports.js).
Every corrected-contract operation has a matching registration. The dynamic
`/api/boards/:boardId/export/${format}` registration supplies the remaining
operation with 40 values. Native catalogs contain 49 import sources, 48 board
export formats, 45 mass-export variants and 21 chart keys.

| Surface | Decision and matching source |
| --- | --- |
| Native Wekan REST | Included: resource management, authentication, explicit authentication OPTIONS, schema-upgrade JSON, Trello HTTP import, raw ZIP import, calendar/chart/mass exports, bot reporting and incoming-email hooks. |
| Dynamic external formats | Included through one method/path operation. [Formatters](https://github.com/wekan/wekan/blob/v12.57/models/lib/externalExportFormatters.js) and [parsers](https://github.com/wekan/wekan/blob/v12.57/models/lib/externalParsers.js) define native encoding and selection. |
| Optional compatibility APIs | Excluded from typed support as selected: [Trello](https://github.com/wekan/wekan/blob/v12.57/server/compatApi/trello.js) 21 operations, [Jira](https://github.com/wekan/wekan/blob/v12.57/server/compatApi/jira.js) 14, [GitHub/Gitea](https://github.com/wekan/wekan/blob/v12.57/server/compatApi/githubLike.js) 11 each: 57 total. |
| Amazon SES/SNS | `POST /api/inbound-email/ses` is intentionally excluded from typed CLI support. The reference retains the native route with `x-cli-excluded` and its reason; it is not counted as an implementation gap. |
| DDP/browser protocols | UI Meteor methods, publications and browser navigation are outside REST. Fixtures use DDP only where needed to produce native UI-created records. |
| Federation/discovery | The five [ActivityPub/WebFinger](https://github.com/wekan/wekan/blob/v12.57/server/activityPub.js) actor, outbox, followers, inbox and discovery routes speak the separate federation protocol with JSON-LD and HTTP signatures. They are outside Wekan's native REST management contract. |
| Identity callbacks | [SQRL](https://github.com/wekan/wekan/blob/v12.57/server/sqrl.js), [Windows SSO](https://github.com/wekan/wekan/blob/v12.57/server/windowsSso.js) and provider callbacks remain separate authentication protocols. |
| Direct files/browser assets | `/cdn/storage`, `/cfs/files`, `/attachments`, `/avatars`, default avatars, Open Graph pages, manifests and license HTML remain direct delivery. Native attachment API download and export are included separately. |
| Instrumentation/deployment | Prometheus `/metrics`, Nextcloud health and Sandstorm's front door remain instrumentation or deployment integration surfaces. |
| Middleware/generated methods | The `/api` middleware prefix is not an operation. Framework HEAD/OPTIONS are excluded; three explicit authentication OPTIONS operations are included. |

The upstream snapshot is unchanged. The overlay corrects omitted operations,
duplicate operation IDs, path selectors, body wrappers, media, nested response
objects and nullable/omitted properties using this exact baseline. Corrected
YAML and HTML are generated locally and remain ignored.

## Behavior and regression evidence

Typed response objects reject additional fields at every depth. Contract tests
also reject invalid types, missing required values and malformed acknowledgments.
Dynamic raw API output preserves the complete payload.
Supported provider email hooks retain native signatures and body bytes.
Successful transport and strict receipt decoding are covered by contract tests,
and the live suite verifies incoming-email and bounce submissions.

The [compatibility ledger](compatibility.md) reconciles all 63 historical cases:
18 fixed, 45 surviving or partially fixed, plus seven newly verified defects.
Positive regressions replace fixed-defect assertions. Expected server failures
have explicit status, persisted-state, byte or artifact assertions. No case is
hidden by relaxed decoding, a skipped assertion, retries or emulation.

Every live suite runs against a fresh disposable Compose project. Verification
covers permissions with separate actors, independent persisted reads, actual
attachment bytes, archive entries/CRCs, native artifacts, all source/format
catalogs and remaining no-op or partial-write behavior. An expected server
failure passes only when its failure and persisted effects match.

Ordinary CI retains Linux, macOS and Windows coverage. Live CI uses isolated
Linux Compose stacks; native Windows credential-store verification remains
conditional on Windows.

## Verification

The 23 October 2026 audit ran all **32 live suites**, each selected exactly once
against a fresh disposable v12.57 stack. The subsequent corrections passed
**753 unique non-live tests**: 368 library, 101 CLI, 282 contract (including the
four generated-contract checks), and two live-matrix inventory checks. The two
ignored library subprocess helpers are exercised by their parent tests.

All three expanded live suites also passed after the corrections: rules,
attachments, and native v12.57 extensions. Each used a fresh isolated stack.
Upload regressions verify W12-006's valid-token HTTP 401 and W12-007's eventual
record removal and orphaned bytes, with bounded native-validation polling
before fixture cleanup. All diagnostic and test stacks and volumes were removed.

Expanded regressions independently check rule enablement and native null
coercion, item deadlines, card section ordering, list archival cascades,
ordinary-user and denied-role mutations, administrator refusals, exact problem
filtering/pagination and acknowledgment counts. Native metadata producers cover
announcements, column sets, WIP groups, whiteboard strokes, custom-field links,
notification delivery, role exceptions, disabled list synchronization, web push,
and ntfy. Populated reads also exercise nested unknown-field rejection.
Passkey and SQRL records use source-checked stored fixtures; their authenticator
protocols are outside these CLI lifecycle tests.

Formatting, Clippy with warnings denied, overlay validation and Swagger
validation passed. The corrected contract has zero Swagger validation errors.
Redocly 2.46.0 generated its corrected HTML reference.
Coverage and parser inventories confirm 168 unique native operations: 167
implemented and one intentionally excluded, with 166 leaf commands. The 52
active defect annotations match the maintained report, including W12-006's
misclassified HTTP 401 after a raw background upload has saved its file, and
W12-007's deferred removal after filename normalization.

Audit and correction evidence is retained under
`target/audit-contract-fixes-20261023/`; generated logs and reference artifacts
are ignored by Git. The original migration evidence remains under
`target/upgrade-v12.57/matrix-final/`.

The untouched upstream specification's SHA-256 is
`cb7a0c87fc17ef14336e52272cfa898a6869037e68e81d6647d144052acb1232`.
This verification ran on Windows with isolated Linux Wekan containers.
Cross-platform CLI CI remains configured; this local run does not claim CLI
execution on Linux or macOS. Excluding typed SES support removes the need for
an AWS-signed message in the supported live-test matrix.

The 24 October 2026 corrections passed **760 unique non-live tests**: 370 library,
101 CLI, 287 contract (including four generated-contract checks), and two live
inventory checks. Five affected live suites passed on fresh isolated v12.57
stacks: attachments, custom fields, native bot/email hooks, users, and exports.
The custom-field suite includes real Jira imports and ordinary-user JSON export
with the new settings; bot reports compare native and CLI persisted state for
omission and null. Upload receipts now distinguish acceptance from unconfirmed
availability, and the known background-helper failure retains its native 401
without credential-replacement advice. W11-015 is retained with its observed
GET 500 and empty PUT 200 behavior. This correction run did not repeat all 32
live suites. Live and generated-contract logs are retained under
`target/audit-contract-fixes-20261024/`.
