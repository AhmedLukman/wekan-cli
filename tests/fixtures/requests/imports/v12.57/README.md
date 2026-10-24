These seven minimal documents cover native import sources without a matching
board export format. Every document imports one `Catalog card` through the
production CLI, followed by independent database and typed board reads.

Shapes come from Wekan v12.57 commit
`647cbd111f43b4c936af2e03480f04cb8e2ef39e`,
`models/lib/planeFormat.js`, `plankaApiFormat.js`, `mstodoApiFormat.js`,
`kanbanflowApiFormat.js`, `basecampApiFormat.js`, `googletasksApiFormat.js`,
`googlekeepApiFormat.js`, and their matching upstream tests. Plane JSON is
sent as native text; the other documents use the API-fetch envelope consumed
by the matching importer. No external credentials or services are required.
