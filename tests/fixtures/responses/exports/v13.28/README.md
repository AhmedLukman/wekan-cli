The `live` artifacts came from a Wekan v13.28 Docker server with a board, list, card and comment. The `rich` and `minimal` artifacts came from the tagged formatters with populated and restricted inputs. The Scrum transfer fixture exercises the native sprint, release and event projection in the live catalog.

The four `dependencies` fixtures came from the v13.28 Taskwarrior, Asana, OpenProject and Jira formatters. Prepare blocks Review, Review blocks Deploy, Prepare duplicates Deploy, and Review is related to Refine. Standalone has no links. Only Deploy has a comment, so the OpenProject fixture covers relations with and without activities.

[Wekan v13.28 formatters](https://github.com/wekan/wekan/blob/v13.28/models/lib/externalExportFormatters.js)
