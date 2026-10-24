# Wekan compatibility

Wekan **v12.57** is the sole supported server baseline. The upstream commit is
[`647cbd111f43b4c936af2e03480f04cb8e2ef39e`](https://github.com/wekan/wekan/tree/v12.57).
The [Compose stack](../compose.yaml) pins the verified image digest and MongoDB
replica set. Wekan CLI 2.0.0 supports this exact server baseline.

The corrected native REST contract has 168 method/path operations: 167 are mapped
in [API coverage](api-coverage.md), and the Amazon SES/SNS endpoint is intentionally
excluded from typed CLI support. Dynamic paths cover 49 import sources,
40 external export formats and 21 chart types. Native board formats, scoped
selection, files and provider email hooks retain their native encodings.
The 57 optional Trello/Jira/GitHub/Gitea compatibility operations are excluded
from typed support by design; the raw API command preserves their full payloads.
See [the route audit](interface-audit.md) for protocol and file-serving exclusions.

Typed commands reject unknown response fields at every object boundary. New
fields are mapped only after checking matching-version producers and live
presence/omission behavior. The CLI does not emulate remaining upstream defects
or silently switch requests, authentication modes or upload transports.

The maintained active defect report is
`C:\Users\itsah\Desktop\wekan-v12.57-defects.md`.
The v11.06 report is historical. Fixed entries are removed from the active
report; this ledger retains their identifiers and the positive regression.

## Historical defect reconciliation

All 63 historical cases have a pinned-source disposition and live evidence.
Eighteen are fixed; 45 retain a defect, including partially repaired cases whose
remaining behavior is described below. Seven newly verified defects use W12 IDs.

| ID | Disposition | v12.57 behavior | Evidence |
| --- | --- | --- | --- |
| W11-001 | Fixed | User creation awaits the write and returns its real ID; duplicate identities fail. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/users.rs) |
| W11-002 | Fixed | Board enumeration authentication failures use HTTP 401/403. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/boards.rs) |
| W11-003 | Fixed | Default native import failures use HTTP 500 instead of success acknowledgments. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/imports.rs) |
| W11-004 | Fixed | Source-specific import exceptions use failure statuses and safe JSON errors. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/imports.rs) |
| W11-005 | Fixed | ZIP export produces a complete archive with independently checked attachment bytes. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/server/ExporterZip.js), [live regression](../tests/e2e/exports.rs) |
| W11-006 | Survives | JSON attachment/background upload and attachment copy/move return HTTP 408 because the stream was already consumed. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/attachmentApi.js), [live regression](../tests/e2e/attachments.rs) |
| W11-007 | Survives | JSON upload/background and copy insertion still call unavailable server Attachments.insertAsync; an exact non-JSON diagnostic payload reaches HTTP 500. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/attachmentApi.js), [live regression](../tests/e2e/attachments.rs) |
| W11-008 | Partly fixed | Missing-board XLSX export still returns HTTP 200 plain text; scoped card missing-resource guards now return 404. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/exportExcel.js), [live regression](../tests/e2e/exports.rs) |
| W11-009 | Fixed | Disabled users cannot establish new REST login sessions; existing sessions are revoked. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/apiAuthRoutes.js), [live regression](../tests/e2e/users.rs) |
| W11-010 | Fixed | User deletion checks existence, awaits removal and returns 404 for a missing user. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/users.rs) |
| W11-011 | Survives | List/card/swimlane/custom-field/dropdown/integration deletions can acknowledge an unmatched selector; wrong-list card deletion can remove children while retaining the parent. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/cards.rs) |
| W11-012 | Survives | Ownership transfer starts board updates without awaiting completion; the returned board list is a request acknowledgment. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/users.rs) |
| W11-013 | Fixed | Board creation deliberately makes its authenticated creator an active administrator; obsolete initial-member flags have been removed from the CLI. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/boards.rs) |
| W11-014 | Fixed | Board creation uses the authenticated creator rather than an arbitrary supplied owner; the obsolete CLI owner option is removed. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/boards.rs) |
| W11-015 | Partly fixed | Missing-user GET now returns HTTP 500 with a safe error instead of an empty success object; missing-user PUT still returns an empty HTTP 200 body. Neither returns 404. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js#L2570-L2630), [live regression](../tests/e2e/users.rs) |
| W11-016 | Survives | Single-card creation ignores body parentId; bulk creation and explicit parent updates are separate native capabilities. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/cards.rs) |
| W11-017 | Partly fixed | Falsy title/description/sort and the misspelled overtime update remain ineffective. Zero spent time and date clearing now persist. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/cards.rs) |
| W11-018 | Survives | Missing scoped-card GET still returns an empty HTTP 200 body. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/historical_defects.rs) |
| W11-019 | Partly fixed | Missing/wrong-scope card PUT now returns 404, but archived-card title updates still acknowledge without changing the archived card. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/historical_defects.rs) |
| W11-020 | Survives | Single-card creation stores nonexistent list or swimlane references; independent global card reads confirm the invalid relationships. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/historical_defects.rs) |
| W11-021 | Survives | A card update can save its title before rejecting a later invalid movement field. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/cards/extensions.rs) |
| W11-022 | Partly fixed | Board domain mutations still lack the DDP identity needed by setBoardDomains; valid site-admin requests now return HTTP 500 and leave domains unchanged. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/board_settings.rs) |
| W11-023 | Survives | Attachment list prefix registration ignores deeper lane/list/card selectors and returns the board collection. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/attachmentApi.js), [live regression](../tests/e2e/attachments.rs) |
| W11-024 | Survives | Administratively disabled JSON export is wrapped as HTTP 500 Export failed instead of a permission response. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/export.js), [live regression](../tests/e2e/historical_defects.rs) |
| W11-025 | Survives | Raw ZIP checklistId is accepted but ignored; populated imports retain board-level lanes/lists/cards/checklists/comments. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/server/scopedImporter.js), [live regression](../tests/e2e/imports.rs) |
| W11-026 | Survives | Public CSV export ignores fields while the authenticated private branch applies recognized selections. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/export.js), [live regression](../tests/e2e/exports.rs) |
| W11-027 | Fixed | Card Excel uses native date cells with the same UTC serial in both public and private workbooks; the former date-text discrepancy is gone. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/server/renderCardDocumentExcel.js), [live regression](../tests/e2e/exports.rs) |
| W11-028 | Survives | Card XLSX selections omitting board-info dereference null list/swimlane records; the started response is aborted and the CLI reports a transport failure. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/server/ExporterExcelCard.js), [live regression](../tests/e2e/exports.rs) |
| W11-029 | Partly fixed | List-move destination filtering is repaired, but populated moves still mutate the first card before HTTP 500, retain the source list and ignore position. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/lists.js), [live regression](../tests/e2e/transfers.rs) |
| W11-030 | Survives | Dependency validation and missing-card failures become HTTP 500; unmatched edge PUT/DELETE can acknowledge the source ID. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/dependencies.js), [live regression](../tests/e2e/dependencies.rs) |
| W11-031 | Survives | Removing a nonexistent card member/assignee dereferences the missing user's username and returns HTTP 500. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/cards.js), [live regression](../tests/e2e/memberships.rs) |
| W11-032 | Fixed | ZIP attachment import completes, preserves independently downloaded bytes and removes its temporary archive. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/lib/importAttachmentStream.js), [live regression](../tests/e2e/imports.rs) |
| W11-033 | Survives | Missing/wrong-state scoped card archive/restore returns HTTP 500 rather than 404. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/cards/archival.rs) |
| W11-034 | Survives | Missing/wrong-scope checklist GET still returns HTTP 500 rather than 404. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/checklists.js), [live regression](../tests/e2e/checklists.rs) |
| W11-035 | Survives | Checklist deletion leaves item documents readable and mutable under the original checklist ID. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/checklists.js), [live regression](../tests/e2e/checklists.rs) |
| W11-036 | Fixed | Read-only and other non-writable roles are denied checklist/item mutations with HTTP 403; each denial is tested with an isolated actor. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/checklistItems.js), [live regression](../tests/e2e/checklists.rs) |
| W11-037 | Survives | Checklist creation and item updates can fail after earlier inserts/completion changes persist. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/checklistItems.js), [live regression](../tests/e2e/checklists.rs) |
| W11-038 | Survives | Direct item updates bypass completion/activity hooks; deleting an item leaves earlier item activities. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/checklistItems.js), [live regression](../tests/e2e/checklists.rs) |
| W11-039 | Survives | Cross-board card moves retain source-board IDs on checklist and item documents. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/checklists.rs) |
| W11-040 | Survives | A custom-field update can save its name before a later invalid settings write fails. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/customFields.js), [live regression](../tests/e2e/custom_fields.rs) |
| W11-041 | Survives | REST alwaysOnCard writes bypass population of the field onto existing cards. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/customFields.js), [live regression](../tests/e2e/custom_fields.rs) |
| W11-042 | Survives | Card custom-field assignment for missing/wrong-list/archived cards still returns HTTP 500. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/customFields.js), [live regression](../tests/e2e/custom_fields.rs) |
| W11-043 | Survives | Linked-card membership changes mutate the source while returning membership information for the link. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/memberships.rs) |
| W11-044 | Survives | Repeated padded board-label names bypass reuse and create duplicate trimmed labels. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/boards.js), [live regression](../tests/e2e/bulk_cards.rs) |
| W11-045 | Survives | Bulk card creation saves earlier cards before a later missing-swimlane error, without per-item results. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/cards.js), [live regression](../tests/e2e/bulk_cards.rs) |
| W11-046 | Survives | Card deletion leaves ordinary attachments visible because the cascade selects top-level cardId instead of meta.cardId. Backing-byte retention alone is normal recoverable deletion. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/cards.js), [live regression](../tests/e2e/bulk_cards.rs) |
| W11-047 | Fixed | Board/card/list/swimlane deep copies await file copying and retain independently downloaded bytes. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/lib/fileStoreStrategy.js), [live regression](../tests/e2e/transfers.rs) |
| W11-048 | Survives | A populated cross-board swimlane move creates destination resources and moves a card before HTTP 500, leaving the source lane. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/swimlanes.js), [live regression](../tests/e2e/transfers.rs) |
| W11-049 | Survives | A successful swimlane move can asynchronously insert an additional deterministic default lane despite existing active lanes. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/swimlanes.js), [live regression](../tests/e2e/transfers.rs) |
| W11-050 | Survives | Missing/wrong-scope integration updates acknowledge the requested ID without a matched write. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/integrations.js), [live regression](../tests/e2e/integrations.rs) |
| W11-051 | Survives | An invalid integration URL can fail after title/enabled changes have persisted. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/integrations.js), [live regression](../tests/e2e/integrations.rs) |
| W11-052 | Survives | Missing/wrong-scope integration GET and activity mutations return an empty HTTP 200 body. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/integrations.js), [live regression](../tests/e2e/integrations.rs) |
| W11-053 | Fixed | The startup hook starts SyncedCron and scheduled rules execute, persist last-run metadata and change card state. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/cron/syncedCron.js), [live regression](../tests/e2e/rules.rs) |
| W11-054 | Fixed | REST-created and updated button rules synchronize denormalized button type/label metadata. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/lib/ruleButtonMetadata.js), [live regression](../tests/e2e/rules.rs) |
| W11-055 | Fixed | Administrator authorization failures use native HTTP 401/403 rather than embedded success responses. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/apiMiddleware.js), [live regression](../tests/e2e/admin_people.rs) |
| W11-056 | Survives | Missing organization/team feature updates return an empty HTTP 200 body. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/org.js), [live regression](../tests/e2e/admin_people.rs) |
| W11-057 | Survives | Private CSV/TSV (and board XLSX) exports reject ordinary members' Bearer authentication while explicit native query authentication succeeds. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/export.js), [live regression](../tests/e2e/exports.rs) |
| W11-058 | Partly fixed | Raw-token revocation acknowledges without hashing/revoking the stored hash. Delete-all now awaits an empty token array and actually revokes sessions. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/user_tokens.rs) |
| W11-059 | Fixed | Token creation checks the user exists and returns 404 without issuing a secret for an unknown selector. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/user_tokens.rs) |
| W11-060 | Survives | Late Trello JSON/ZIP failures leave persisted boards without returning their IDs; ZIP still returns explicit per-board errors. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/importTrelloZip.js), [live regression](../tests/e2e/trello_import.rs) |
| W11-061 | Fixed | List copy supports omitted destination lanes and same-board copies, creates a new list ID and includes source cards across lanes. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/lists.js), [live regression](../tests/e2e/transfers.rs) |
| W11-062 | Survives | Ownership selection matches userId and administrator flags on different members, transferring an unrelated qualifying board and potentially duplicating caller memberships. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/models/users.js), [live regression](../tests/e2e/users.rs) |
| W11-063 | Survives | GridFS-only JSON board export emits empty file text, and individual attachment JSON export emits null despite successful byte downloads; ZIP uses the storage strategy correctly. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/exporter.js), [live regression](../tests/e2e/exports.rs) |

## New v12.57 defects

| ID | Disposition | v12.57 behavior | Evidence |
| --- | --- | --- | --- |
| W12-001 | New | Dropdown option name PUT fails in the positional field-write hook and leaves the value unchanged. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/adminOnlyFieldWrites.js), [live regression](../tests/e2e/custom_fields.rs) |
| W12-002 | New | A legitimate private-board member receives an attachment Bearer-export denial and an ImpersonateBleed account block. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/export.js), [live regression](../tests/e2e/exports.rs) |
| W12-003 | New | Mass CSV/SCSV/TSV returns a ZIP containing only skipped.txt and disables its exporting actor on its own non-plain list objects. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/exportAllBoards.js), [live regression](../tests/e2e/catalog.rs) |
| W12-004 | New | Native ZIP export uses filenames without attachment IDs; scoped ZIP import cannot resolve those entries and reports zero attachments after importing the cards. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/importZip.js), [live regression](../tests/e2e/exports.rs) |
| W12-005 | New | Raw ZIP newBoard=1 starts its tracked importer outside the DDP identity context; valid native exports return HTTP 500 import-failed in default and me member modes. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/importZip.js), [live regression](../tests/e2e/exports.rs) |
| W12-006 | New | Raw background uploads persist the image, then misclassify the background-selection helper failure as HTTP 401 while the credential remains valid and the board background stays unchanged. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/server/routes/attachmentApi.js#L471-L608), [live regression](../tests/e2e/attachments.rs) |
| W12-007 | New | Upload filename normalization leaves the deferred validator's top-level path stale; a successful raw card upload can lose its attachment record and leave its renamed file orphaned. | [v12.57 source](https://github.com/wekan/wekan/blob/v12.57/models/attachments.server.js#L135-L225), [live regression](../tests/e2e/attachments.rs) |
