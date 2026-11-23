# Wekan v13.28 interface and route audit

Wekan CLI **3.0.0** exposes **168 leaf commands in 19 families**.
The corrected contract contains **169 native method/path pairs**. The CLI implements 168.
The remaining entry is the Amazon SES/SNS callback.
[Command inventory](command-contract.md) · [API coverage and arguments](api-coverage.md)

The upstream Swagger contains 157 operations. The overlay describes native routes
omitted by the generator, including the file staging endpoint in this release.
Each operation is counted once even when several commands or format choices select it.
The Compose image reports `v13.28.0`. The tagged source is commit
[`9da7639`](https://github.com/wekan/wekan/tree/v13.28).

## Route scope

| Surface | Decision and matching source |
| --- | --- |
| Native Wekan REST | Included: resource management, authentication, explicit authentication OPTIONS, schema-upgrade JSON, Trello HTTP import, raw ZIP import, calendar/chart/mass exports, bot reporting and incoming-email hooks. |
| Dynamic external formats | Included through one method/path operation. [Formatters](https://github.com/wekan/wekan/blob/v13.28/models/lib/externalExportFormatters.js) and [parsers](https://github.com/wekan/wekan/blob/v13.28/models/lib/externalParsers.js) define native encoding and selection. |
| Optional compatibility APIs | Trello, Jira, GitHub and Gitea emulation use separate protocol surfaces. [Tagged server adapters](https://github.com/wekan/wekan/tree/v13.28/server/compatApi) |
| Amazon SES/SNS | `POST /api/inbound-email/ses` is intentionally excluded from typed CLI support. The reference retains the native route with `x-cli-excluded` and its reason; it is not counted as an implementation gap. |
| DDP/browser protocols | UI Meteor methods, publications and browser navigation are outside REST. Fixtures use DDP only where needed to produce native UI-created records. |
| Federation/discovery | The five [ActivityPub/WebFinger](https://github.com/wekan/wekan/blob/v13.28/server/activityPub.js) actor, outbox, followers, inbox and discovery routes speak the separate federation protocol with JSON-LD and HTTP signatures. They are outside Wekan's native REST management contract. |
| Identity callbacks | [SQRL](https://github.com/wekan/wekan/blob/v13.28/server/sqrl.js), [Windows SSO](https://github.com/wekan/wekan/blob/v13.28/server/windowsSso.js) and provider callbacks remain separate authentication protocols. |
| Direct files/browser assets | `/cdn/storage`, `/cfs/files`, `/attachments`, `/avatars`, default avatars, Open Graph pages, manifests and license HTML remain direct delivery. Native attachment API download and export are included separately. |
| Instrumentation/deployment | Prometheus `/metrics`, Nextcloud health and Sandstorm's front door remain instrumentation or deployment integration surfaces. |
| Middleware/generated methods | The `/api` middleware prefix is not an operation. Framework HEAD/OPTIONS are excluded; three explicit authentication OPTIONS operations are included. |

## Format coverage

The catalogs contain 85 import sources, 67 external single-board formats, 75
single-board export formats in total, 71 mass-export formats and 33 chart keys.
The Trello ZIP export is available for individual boards. Import staging accepts
raw bytes and returns an upload ID and size. REST board import still takes its
input separately. [Imports](imports.md) · [Exports](exports.md)

## Verification

The executable tests walk the parser tree and compare every leaf with the command
inventory. The contract tests compare coverage rows with the generated Swagger.
Typed responses reject unknown fields, including fields in nested objects.
The live suite exercises production dispatch on disposable MongoDB 7 replica
sets. The metadata regression covers populated card-list projections, subtask
creation, board display settings and stored card, list and user metadata.

The [compatibility ledger](compatibility.md) records the disposition and evidence
for all 70 historical cases and the new release findings. It distinguishes
release-specific live checks from source review and earlier verification.

All 33 live matrix entries passed on disposable v13.28 stacks. The catalog suite
checked the 75 single-board artifacts, 71 mass formats, both output formats for
each of the 33 charts, and every import source through staging and board import.
WordPress and Blogger XML round trips, and Telegram and Discord ZIP imports,
returned the native failures recorded as W13-001 and W13-002.

Of the 70 historical cases, 50 are classified as fixed, 14 remain open and
five are partly fixed. The historical account lockout on private attachment
export still awaits release-specific live evidence. The active report lists
the 19 continuing or partly fixed historical cases and four v13.28 findings.
The ownership tests cover self-targeted requests (W13-003) and repeated
transfers involving duplicate membership entries (W13-004).
