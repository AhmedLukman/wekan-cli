# Comments

The singular `wekan comment` command family covers every card-comment
operation exposed by the Wekan v12.57 REST API:

```console
wekan comment list --board <BOARD_ID> --card <CARD_ID>
wekan comment get <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID>
wekan comment create --board <BOARD_ID> --card <CARD_ID> --text <TEXT>
wekan comment update <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID> --text <TEXT>
wekan comment delete <COMMENT_ID> --board <BOARD_ID> --card <CARD_ID> [--yes]
```

Identifiers are trimmed and must not be empty. Comment creation and updating
send the text unchanged and reject text that is blank under
JavaScript's whitespace rules. Wekan trims that whitespace before insertion;
U+0085 (next line) is preserved, including when it is the entire comment, while
U+FEFF is trimmed. The create endpoint accepts only the text; it cannot create
a threaded reply, although `comment get` preserves a returned `parent_id`.

## Stable output

Use `--output json` for stable machine-readable envelopes. Collection, detail,
create/update, and delete data have these shapes:

```json
{"board_id":"board-id","card_id":"card-id","comments":[{"comment_id":"comment-id","text":"Hello","author_id":"user-id"}]}
{"comment_id":"comment-id","board_id":"board-id","card_id":"card-id","text":"Hello","parent_id":null,"created_at":"2030-01-02T03:04:05Z","modified_at":"2030-01-02T03:04:05Z","author_id":"user-id","sync_comment_id":null}
{"board_id":"board-id","card_id":"card-id","comment_id":"comment-id"}
{"board_id":"board-id","card_id":"card-id","comment_id":"comment-id","deleted":true,"delete_mode":"hard"}
```

These objects appear under the usual `{"ok":true,"data":...}` envelope.
Wekan's compact collection fields (`comment` and `authorId`) and full-document
fields (`text` and `userId`) are normalized to the same stable `text` and
`author_id` names. Empty, null, or omitted top-level `parentId` becomes JSON
`null`. All response objects reject unknown fields; IDs must be non-empty,
text must not be blank under JavaScript's whitespace rules, and timestamps
must be RFC3339 date-times. List and get preserve the returned text, including
U+0085.

A missing individual comment produces an empty HTTP 200 response in Wekan
v12.57. The CLI normalizes it to `not_found`. Listing or creating comments for a
nonexistent card, or a card outside the selected board, returns HTTP 404.

## Verified Wekan behavior

List and get use Wekan's board-access check: normal and read-only members may
read, while no-comments, comment-only, and worker roles are rejected. Creation
uses the separate comment capability: normal, comment-only, and worker members
may create comments, while read-only and no-comments members receive HTTP 403.
Per-list role exceptions can also change creation permission. Site administrators
bypass these membership checks. The CLI preserves each native route's checks.

`comment delete` is a hard deletion and requires interactive confirmation or
command-local `--yes`. Authors may delete their own comments. A board admin may
delete another author's comment only when the board does not restrict comment
editing; other members receive a permission error. Wekan returns the card ID
after deletion, so the CLI verifies that ID before reporting the requested
comment as deleted. A malformed or mismatched successful response is reported
with `protocol_error`. A missing board or comment during deletion produces
`not_found` and is known not to have deleted anything. Declining deletion uses
cancellation operation `comment_delete`.

## Editing comments

`wekan comment update COMMENT_ID --board BOARD_ID --card CARD_ID --text TEXT`
replaces comment text through the native PUT operation. Wekan applies its
author, board administrator, and restricted-editing rules. Listing or creating
comments for a nonexistent card returns HTTP 404.
