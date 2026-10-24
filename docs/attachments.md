# Attachments and file transfers

The `attachment` family covers metadata, transfer, and recovery operations in the
corrected Wekan v12.57 contract. The separate
[`board attachment list`](boards.md#attachment-links) command exposes the legacy
board link projection, including backgrounds. Commands require a selected profile and a stored,
unexpired credential. Global options are `--profile`, `--server`, `--output`,
`--allow-insecure-http`, and `--help` (`-h`). Human output is the default.

## Commands

```text
wekan attachment list --board <BOARD_ID> [--swimlane-id <ID>] [--list <ID>] [--card <ID>]
wekan attachment get <ATTACHMENT_ID>
wekan attachment upload <FILE> --board <ID> --swimlane-id <ID> --list <ID> --card <ID> [--file-name <NAME>] [--file-type <MIME>] [--transport json|raw]
wekan attachment download <ATTACHMENT_ID> --destination <PATH> [--overwrite]
wekan attachment copy <ATTACHMENT_ID> --target-board <ID> --target-swimlane-id <ID> --target-list <ID> --target-card <ID>
wekan attachment move <ATTACHMENT_ID> --target-board <ID> --target-swimlane-id <ID> --target-list <ID> --target-card <ID>
wekan attachment delete <ATTACHMENT_ID> [--board <BOARD_ID>] [--yes]
wekan attachment list-deleted --board <BOARD_ID>
wekan attachment restore <ATTACHMENT_ID> --board <BOARD_ID>
wekan attachment upload-background <FILE> --board <ID> [--file-name <NAME>] [--file-type <MIME>] [--transport json|raw]
wekan attachment download-background --board <ID> --destination <PATH> [--overwrite]
```

`--list` requires `--swimlane-id`; `--card` additionally requires `--list`.
Each additional selector selects its corresponding deeper REST endpoint.
Selectors are nonblank, trim JavaScript whitespace, and reject `.` and `..`
because URL normalization cannot preserve those path segments.

## Pinned server behavior

Wekan v12.57 registers the board-only list prefix before the deeper paths. Every
listing returns all board attachments, including requests with nonexistent deeper
selectors. The CLI retains those results and reports `effective_scope: "board"`;
`requested_scope` is `board`, `swimlane`, `list`, or `card`. Human output warns
when deeper selectors were supplied. It does not emulate filtering.

Normal JSON upload, background-upload, copy, and move requests return HTTP 408
after approximately 30 seconds because the global parser has already consumed
the request stream. The CLI uses 60-second deadlines for these four commands
and reports the actual server failure. For uploads and background uploads,
`--transport raw` explicitly selects the native octet-stream endpoint mode;
metadata travels in the query and the body contains the original bytes. The
default remains JSON, and failures never trigger an automatic transport change. All other attachment requests retain the
30-second client deadline. Upload/background/copy insertion also contains a
separate upstream `Attachments.insertAsync` defect if that path is reached.
Raw background uploads save the attachment and then return HTTP 401 because
the server's background-selection helper calls an unavailable user method
(W12-006). The board's background remains unchanged, while the uploaded file
and its metadata remain stored after native content validation. This helper
failure is misclassified as an authentication rejection; the same credential
continues to work on authenticated reads. The CLI reports the native status without
retrying or selecting the background through another operation.
For this verified helper failure, the CLI returns `server_error` with the native
HTTP 401 and error text, `mutation_attempted: true`, and `retry_safe: false`.
It does not advise replacing the valid credential. Actual authentication
failures continue to return `authentication_rejected`.
Native upload filename normalization can trigger another deferred failure
(W12-007): a filename longer than 30 characters is truncated, but validation
reads its old filesystem path and deletes the attachment record. A raw card
upload can therefore return HTTP 200 and an ID that subsequently returns 404,
while the renamed file remains orphaned on disk. The CLI sends the requested
filename unchanged and does not retry or truncate it locally.
See the [pinned route implementation](https://github.com/wekan/wekan/blob/v12.57/server/routes/attachmentApi.js).

## Upload contents

Use a regular file path or `-` for binary stdin. Stdin requires `--file-name`;
file uploads default to the UTF-8 basename and allow an explicit name override.
Supplied names and MIME types are sent unchanged. Omitted MIME types preserve
the server defaults: `application/octet-stream` for card attachments and
`image/png` for backgrounds. The CLI performs no MIME detection. Wekan can
normalize stored filenames, including its 30-character length cap.
Raw background uploads send the default `image/png` as `fileType` query metadata,
while retaining the octet-stream HTTP content type required by that transport.

The CLI acquires at most 64 MiB plus one byte to enforce its 64 MiB input limit,
then base64-encodes the contents for JSON transport or sends the original bytes
for raw transport. Larger input returns `invalid_input`,
exit 2, without an HTTP mutation. Empty files are passed through to Wekan.
The server has independent administrator transfer limits and a 50 MiB global
JSON body limit; their failures are retained.

## Downloads and replacement

File downloads require an explicit destination. Its parent directory must exist.
The server's filename is metadata only and never selects a local path. Existing
destinations are protected unless `--overwrite` explicitly permits replacement.
Downloads validate the complete strict JSON response and base64 contents before
writing a temporary file beside the destination and publishing it. Transfer or
staging failures preserve the existing file and remove temporary files.

Downloads allow up to 96 MiB of response JSON to accommodate the server's 64 MiB
file cap and base64 expansion. Metadata commands and non-200 error bodies retain
the existing 1 MiB response limit. Downloads buffer and decode before writing;
they do not stream unvalidated server content.

For pipelines, use `--destination -` with `--output raw`:

```console
wekan --output raw attachment download ATTACHMENT_ID --destination - > attachment.bin
wekan --output raw attachment download-background --board BOARD_ID --destination - > background.png
```

Raw stdout receives decoded bytes with no newline or metadata. Raw mode rejects
file destinations; human/JSON modes reject stdout destinations. `--overwrite`
conflicts with stdout. Shell redirection itself can truncate a file before the
CLI runs; use a file destination to obtain the CLI's replacement protection.

## Results and errors

Lists include requested selector IDs, `requested_scope`, `effective_scope`,
`count`, and typed `attachments`. Metadata includes IDs, filename, size, MIME,
storage backend, optional location/timestamp/image fields, and typed versions.
Missing optional properties become null in stable output; unmapped fields or
invalid mapped values produce `protocol_error` (exit 4). Stable output nulls can
represent omitted upstream properties even where an explicit null is not allowed
in the typed response. Scoped listings expose `requested_scope` and
`effective_scope`.

Saved downloads return `attachment_id`, `file_name`, `file_size`, `file_type`,
`storage_backend`, `background_image_url` (null for ordinary files), `destination`,
`bytes_written`, and `file_saved: true`, without base64 contents. `file_size`
preserves server metadata; `bytes_written` records the actual decoded byte count.
File I/O failures use `file_io_error` (exit 4). If success reporting fails after
publication, error details retain `file_saved: true` and `destination`.

Card upload receipts return `attachment_id`, `file_name`, `file_size`,
`storage_backend`, `server_message`, `request_accepted: true`, and
`upload_confirmed: false`. The filename, size, backend, and server message are
receipt metadata; they do not confirm completed validation, storage migration,
or final file availability. Human output explicitly reports acceptance with
unconfirmed availability. No follow-up request or automatic retry is performed.
The W12-007 live regression confirms that this same accepted receipt can precede
the attachment's eventual removal.

Other mutations return their mapped response fields. Delete requires confirmation or
`--yes`; declining is an exit-0 cancellation with operation `attachment_delete`.
Deletion re-reads and checks credential identity and expiry after confirmation,
immediately before sending. It holds the credential guard until the HTTP request finishes.
Removed, replaced, or expired credentials stop deletion without HTTP.
Uploads, copy, move, background changes, and local `--overwrite` do not require
an additional confirmation. Mutations are never automatically retried. Transport,
protocol, and ambiguous server failures retain the existing outcome diagnostics.

`attachment export` writes native attachment JSON through the shared
[export interface](exports.md). Its `file` field contains base64 data or `null`.
Wekan v12.57's exporter reads a filesystem path; a GridFS-only attachment can
export `file: null` even when its normal download succeeds. Inspect that field
before treating the saved JSON as a backup of the bytes. A missing attachment
exports `{}`.
Legacy board-attachment listing and administrator storage settings use separate
operations.

## Recoverable deletion

`attachment delete ID` uses the legacy endpoint. Supplying `--board BOARD_ID`
selects the board-scoped deletion endpoint explicitly. Both retain the file for
restoration; confirmation describes recoverable deletion. `list-deleted`
returns the board's deleted attachment metadata. `restore` returns `restored`
to distinguish restoration from an already restored file. Restoration preserves
the original file bytes and does not permanently remove stored content.
