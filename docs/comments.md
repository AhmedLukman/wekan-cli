# Comments

The singular `wekan comment` command family covers every card-comment
operation exposed by the Wekan v11.06 REST API:

```console
wekan comment list --board <BOARD_ID> --card <CARD_ID>
wekan comment get <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID>
wekan comment create --board <BOARD_ID> --card <CARD_ID> --text <TEXT>
wekan comment delete <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID> [--yes]
```

Wekan does not expose a REST endpoint for editing a comment, so this family has
no `update` command. Identifiers and comment text are trimmed and must not be
empty. The create endpoint accepts only the text; it cannot create a threaded
reply, although `comment get` preserves a returned `parent_id`.

## Stable output

Use `--output json` for stable machine-readable envelopes. Collection, detail,
create, and delete data have these shapes:

```json
{"board_id":"board-id","card_id":"card-id","comments":[{"comment_id":"comment-id","text":"Hello","author_id":"user-id"}]}
{"comment_id":"comment-id","board_id":"board-id","card_id":"card-id","text":"Hello","parent_id":null,"created_at":"2030-01-02T03:04:05Z","modified_at":"2030-01-02T03:04:05Z","author_id":"user-id"}
{"board_id":"board-id","card_id":"card-id","comment_id":"comment-id"}
{"board_id":"board-id","card_id":"card-id","comment_id":"comment-id","deleted":true,"delete_mode":"hard"}
```

These objects appear under the usual `{"ok":true,"data":...}` envelope.
Wekan's compact collection fields (`comment` and `authorId`) and full-document
fields (`text` and `userId`) are normalized to the same stable `text` and
`author_id` names. Empty, null, or omitted top-level `parentId` becomes JSON
`null`. All response objects reject unknown fields; IDs and text must be
non-empty and timestamps must be RFC3339 date-times.

A missing individual comment produces an empty HTTP 200 response in Wekan
v11.06. The CLI normalizes it to `not_found`. Listing a nonexistent card
returns an empty collection because Wekan does not verify card existence.

## Verified Wekan behavior

Wekan's create route also does not verify card existence and can insert an
orphan comment for a nonexistent card. The CLI mirrors the endpoint without a
preflight or cleanup workaround.

Comment routes use Wekan's general board-access check. Active read-only members
are accepted, while members marked no-comments, comment-only, or worker are
rejected; site administrators bypass the membership condition. These
counterintuitive v11.06 rules are preserved rather than emulated differently
by the CLI.

`comment delete` is a hard deletion and requires interactive confirmation or
command-local `--yes`. Authors may delete their own comments. A board admin may
delete another author's comment only when the board does not restrict comment
editing; other members receive a permission error. Wekan returns the card ID
after deletion, so the CLI verifies that ID before reporting the requested
comment as deleted. A malformed or mismatched successful response is reported
with `outcome_unknown: true`.
