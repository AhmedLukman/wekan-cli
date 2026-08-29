# Raw API requests

`wekan api request` is the low-level escape hatch for Wekan operations that do
not yet have a dedicated CLI command. It sends one same-origin request to the
server in an existing selected profile and does not map or discard response
fields.

```console
wekan api request GET /api/boards
wekan api request POST /api/boards/BOARD/cards/CARD/checklists --json '{"title":"Release"}' --yes
wekan --output raw api request GET /api/boards/BOARD/lists/LIST/cards/CARD/exportPDF --auth-token-query --timeout 300 > card.pdf
```

## Request target and query

The syntax is `wekan api request METHOD PATH`. `PATH` must begin with exactly
one `/` and is resolved beneath the configured Wekan base URL. For example,
profile server `https://example.test/wekan/` plus `/api/boards` requests
`https://example.test/wekan/api/boards`. Absolute URLs, fragments, dot-segment
traversal, backslashes, and paths outside that configured base are rejected before
credential access or HTTP.

The path may contain a query string. Repeatable `--query NAME=VALUE` arguments
append URL-encoded pairs in order and preserve duplicates. The `authToken`
name is reserved; use `--auth-token-query` so a saved token never has to appear
in process arguments. Managed query-token injection is accepted only for `GET`
requests, but it is not restricted to a hard-coded route list so the raw escape
hatch can reach unmodeled same-origin Wekan endpoints.

## Authentication and headers

By default the command requires a current stored credential and sends it as a
Bearer token. `--auth-token-query` instead sends the same stored token as
Wekan's `authToken` query parameter, which is needed by some v11.06 private
export routes. It is allowed for any `GET` route; callers should prefer the
default Bearer mode unless the endpoint requires query authentication.
`--no-auth` does not read or inject the selected profile's
stored credential. It does not prohibit authentication explicitly supplied by
the caller, such as a `Cookie` header. These two options conflict.

Use repeatable `-H/--header NAME:VALUE` arguments for other request headers.
Authorization, Host, Content-Length, and HTTP hop-by-hop/framing headers are
owned by the transport and cannot be overridden. Header values are never
printed by the CLI as request metadata.

`TRACE`, `TRACK`, and `CONNECT` are rejected. Their reflection or tunneling
semantics are unsafe for a command that can inject managed credentials.

## Request bodies and confirmation

Choose at most one body source:

- `--body TEXT` sends UTF-8 bytes;
- `--body-file PATH` streams file bytes and supplies its exact Content-Length;
- `--body-stdin` streams standard input;
- `--json JSON` validates the value and defaults Content-Type to
  `application/json` when no Content-Type header was supplied.

Only GET, HEAD, and OPTIONS are treated as safe. Every other method requires an
interactive confirmation or command-local `--yes`. JSON output never prompts.
Raw output may prompt only when stdin and stderr are terminals, so a request
whose body is piped through stdin also needs `--yes`. Declining returns the
normal `api_request` cancellation result and sends nothing. `--yes` is rejected
for safe methods.

The default total request timeout is 30 seconds, including response-body
streaming. Use `--timeout SECONDS` to override it for a large export. Connection
setup retains its separate 10-second limit.

## Response output

The command never applies typed Wekan DTO decoding and never treats an error
object embedded in HTTP 200 as an error. HTTP 2xx is success, 3xx is
`unexpected_redirect`, 401 is `authentication_rejected`, 403 is
`permission_denied`, and other non-2xx statuses are `server_error`.

Human output shows the status, all response headers, and a safely rendered
body. JSON output uses this shape inside the standard success envelope:

```json
{
  "ok": true,
  "data": {
    "http_status": 200,
    "mutation_attempted": false,
    "mutation_confirmed": false,
    "headers": [
      {"name": "content-type", "encoding": "text", "value": "application/json"}
    ],
    "body": {"encoding": "json", "value": {"any": "fields are preserved"}}
  }
}
```

Body encoding is `json` for valid JSON, `text` for other UTF-8, and `base64`
for binary bytes. Non-text header values use base64 as well. Repeated header
values remain separate array entries. A non-2xx JSON error stores this same
response object in `error.details.response`.

Human and JSON modes retain the 1 MiB structured-response limit. Use
`--output raw` for larger or byte-sensitive bodies. Exceeding the limit returns
`api_response_too_large`, records the known HTTP status and whether it was
successful, and marks an unsafe request's mutation outcome as unconfirmed. The
same success and retry metadata applies when structured response streaming
fails before the body is complete; these incomplete responses also set
`outcome_unknown: true` for unsafe methods. For every unsafe method,
`mutation_attempted: true` and `mutation_confirmed: false` make the absence of
an authoritative readback explicit. `retry_safe: false` warns that the request
must not be repeated merely because structured rendering failed. Raw mode is
valid only for this command: it
streams an HTTP 2xx body exactly to stdout, or a
non-2xx body exactly to stderr, without adding a newline, headers, or status
text. The exit status still reports success or failure. A mid-stream failure
can leave partial bytes and returns exit 4 without appending a diagnostic to
those bytes.

CLI-generated diagnostics and debug representations remove managed
token-bearing request URLs. This protection applies to every same-origin `GET`
route used with `--auth-token-query`. Raw responses are intentionally unmodified, and
structured responses preserve the same upstream data. A Wekan server, reverse
proxy, or redirect can echo a requested query URL—including `authToken`—in a
response header or body. Callers are responsible for protecting that output
and any upstream access logs.
