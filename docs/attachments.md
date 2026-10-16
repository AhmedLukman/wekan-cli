# Attachments and file transfers

The `attachment` family covers all twelve operations tagged Attachments in the
corrected Wekan v11.06 contract. Commands require a selected profile and a stored,
unexpired credential. Global options are `--profile`, `--server`, `--output`,
`--allow-insecure-http`, and `--help` (`-h`). Human output is the default.

## Commands

```text
wekan attachment list --board <BOARD_ID> [--swimlane-id <ID>] [--list <ID>] [--card <ID>]
wekan attachment get <ATTACHMENT_ID>
wekan attachment upload <FILE> --board <ID> --swimlane-id <ID> --list <ID> --card <ID> [--file-name <NAME>] [--file-type <MIME>]
wekan attachment download <ATTACHMENT_ID> --destination <PATH> [--overwrite]
wekan attachment copy <ATTACHMENT_ID> --target-board <ID> --target-swimlane-id <ID> --target-list <ID> --target-card <ID>
wekan attachment move <ATTACHMENT_ID> --target-board <ID> --target-swimlane-id <ID> --target-list <ID> --target-card <ID>
wekan attachment delete <ATTACHMENT_ID> [--yes]
wekan attachment upload-background <FILE> --board <ID> [--file-name <NAME>] [--file-type <MIME>]
wekan attachment download-background --board <ID> --destination <PATH> [--overwrite]
```

`--list` requires `--swimlane-id`; `--card` additionally requires `--list`.
Each additional selector selects its corresponding deeper REST endpoint.
Selectors are nonblank, trim JavaScript whitespace, and reject `.` and `..`
because URL normalization cannot preserve those path segments.

## Pinned server behavior

Wekan v11.06 registers the board-only list prefix before the deeper paths. Every
listing returns all board attachments, including requests with nonexistent deeper
selectors. The CLI retains those results and reports `effective_scope: "board"`;
`requested_scope` is `board`, `swimlane`, `list`, or `card`. Human output warns
when deeper selectors were supplied. It does not emulate filtering.

Normal JSON upload, background-upload, copy, and move requests return HTTP 408
after approximately 30 seconds because the global parser has already consumed
the request stream. The CLI uses 60-second deadlines for these four commands
and reports the actual server failure. It does not change content types to bypass
the defect or emulate the operation. All other attachment requests retain the
30-second client deadline. Upload/background/copy insertion also contains a
separate upstream `Attachments.insertAsync` defect if that path is reached.
See the [pinned route implementation](https://github.com/wekan/wekan/blob/v11.06/server/routes/attachmentApi.js).

## Upload contents

Use a regular file path or `-` for binary stdin. Stdin requires `--file-name`;
file uploads default to the UTF-8 basename and allow an explicit name override.
Supplied names and MIME types are sent unchanged. Omitted MIME types preserve
the server defaults: `application/octet-stream` for card attachments and
`image/png` for backgrounds. No MIME detection is performed.

The CLI acquires at most 64 MiB plus one byte to enforce its 64 MiB input limit,
then base64-encodes the contents in JSON. Larger input returns `invalid_input`,
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
invalid mapped values produce `protocol_error` (exit 4).

Saved downloads return `attachment_id`, `file_name`, `file_size`, `file_type`,
`storage_backend`, `background_image_url` (null for ordinary files), `destination`,
`bytes_written`, and `file_saved: true`, without base64 contents. `file_size`
preserves server metadata; `bytes_written` records the actual decoded byte count.
File I/O failures use `file_io_error` (exit 4). If success reporting fails after
publication, error details retain `file_saved: true` and `destination`.

Mutations return their mapped response fields plus `mutation_confirmed: false`.
This is an acknowledgment without a readback. Delete requires confirmation or
`--yes`; declining is an exit-0 cancellation with operation `attachment_delete`.
Deletion re-reads and checks credential identity and expiry after confirmation,
immediately before sending. It holds the credential guard until the HTTP request finishes.
Removed, replaced, or expired credentials stop deletion without HTTP.
Uploads, copy, move, background changes, and local `--overwrite` do not require
an additional confirmation. Mutations are never automatically retried. Transport,
protocol, and ambiguous server failures retain the existing outcome diagnostics.

`attachment export` writes native attachment JSON, including base64 data, through
the shared [export interface](exports.md). A missing attachment exports `{}`.
Legacy board-attachment listing and administrator storage settings use separate
operations.
