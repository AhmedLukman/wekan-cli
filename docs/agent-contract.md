# Agent output and error contract

Use `--output json` for deterministic machine-readable output. This topic covers
shared envelopes, streams, decoding, confirmations, and errors. Result fields
and command-specific outcomes belong to the relevant family topic from
`wekan skill list`; for example, read `wekan skill show cards` for card schemas.
Use command-specific `--help` for syntax.

## Streams

- Successful command envelopes are written to stdout and stderr is empty.
- Error envelopes are written to stderr and stdout is empty.
- Every envelope is one JSON object followed by a newline.
- `--help` and `--version` remain human-readable text on stdout.
- Human-mode errors are written to stderr.
- Parser errors honor the last `--output` option before the `--` positional
  separator, including overrides on nested commands.
- Interactive confirmation prompts are written to stderr. JSON output never
  prompts; destructive automation must pass command-local `--yes`.

Stream guarantees assume working output destinations. A write or flush failure
can leave partial or even complete success bytes on stdout and returns exit 4;
an error envelope is delivered on stderr only if that stream remains writable.
Require a successful process status before accepting a complete success result.

Raw API bodies and file bytes have separate stream, validation, and delivery
rules. Read [raw API output](api.md#response-output),
[attachment downloads](attachments.md#downloads-and-replacement), or
[export delivery](exports.md#file-delivery-and-output) for the operation.

## JSON presence and nullability

Success objects normally emit every mapped field, with `null` for unavailable
optional values. Collections use named arrays, including `[]` for an empty
result. Resource contracts document exceptions, nullable arrays/elements, and
fields that preserve omission separately from explicit null.

`false`, zero, empty string, empty array/object, null, and omission are not
interchangeable. Known nullable values and typed unions retain their documented
types. Dynamic dictionary keys are data, not additional schema fields; their
values remain subject to the declared contract.

`error.details` is always an object. Irrelevant or unavailable properties are
omitted rather than universally filled with null.

## Response decoding diagnostics

Strict response-decoding failures use the `protocol_error` code and exit
status 4. Their error details also include:

- `response_path`: a JSON Pointer-style location such as `/wipLimit/enabled`
  or `/0/title`. An empty string identifies the response root.
- `response_error`: `unknown_field`, `missing_field`, `invalid_type`,
  `invalid_value`, or `invalid_json`.

These diagnostics apply to typed successes, authentication responses, and
decoded Wekan error envelopes. For example:

```json
{"ok":false,"error":{"code":"protocol_error","message":"the list response contains unknown field at /wipLimit/future","details":{"response_path":"/wipLimit/future","response_error":"unknown_field","http_status":200}}}
```

Other context and mutation-outcome fields remain applicable. Diagnostics never
include rejected field values or the underlying deserializer's error text.
Field-name segments escape control characters and JSON Pointer metacharacters;
names longer than 80 characters and paths deeper than 16 segments are marked
`[truncated]`. Known secrets are redacted by the command's error mapper.
Semantic validation failures after decoding may omit these two fields.
Raw API output continues to preserve the upstream response without typed decoding.

## Destructive confirmation

Commands that require confirmation accept command-local `--yes`. It suppresses
the prompt and does not establish user authorization. Other commands reject
that flag; use command help to check support.

Human terminals prompt before destructive work. Non-terminal and JSON
invocations without required confirmation return `invalid_input`, exit 2.
Documented target/removability preflights can run first and retain their own
errors. Resource topics describe additional requirements and cancellation
operation names.

Declining an interactive prompt is an exit-0 no-op with this envelope:

```json
{"ok":true,"data":{"cancelled":true,"operation":"card_delete"}}
```

Human output is `Cancelled; no changes made.` The `operation` value identifies
the cancelled command. No destructive request is sent after cancellation.

## Mutation outcomes

An HTTP success status alone does not establish CLI success. Typed decoding,
embedded Wekan errors, and partial batch results can still fail. A successful
result describes the command's documented outcome, which may acknowledge a
request rather than establish a stored change. Submitted-field names describe
inputs rather than verified saved values. Read the resource topic for details.

A failed mutation can already have changed remote or local state, including
after transport, decoding, credential, or output-delivery failures. Inspect
documented outcome details and report uncertainty. Missing details do not prove
that nothing changed or that retrying is safe. Do not blindly repeat mutations
or emulate server behavior.

## Errors

Error envelopes have a stable code, human-readable message, and details object:

```json
{"ok":false,"error":{"code":"transport_error","message":"request failed","details":{"profile":"work"}}}
```

Depending on the error, shared details can include:

- `profile`: the selected profile when target context is relevant;
- `http_status`: the actual HTTP status;
- `wekan_status_code`: an application status serialized inside a response;
- `server_error`, `server_reason`, and `server_message`: native Wekan metadata;
- `server_error_type` and `server_is_client_safe`: native error metadata;
- `response_path` and `response_error`: strict decoding diagnostics above.

Operation-specific fields are documented with the operation: see
[authentication](auth.md#error-details), [profiles](config.md#output-and-errors),
[raw API responses](api.md#response-output),
[file delivery](exports.md#file-delivery-and-output),
[batch results](bulk-cards-and-labels.md#batch-results-and-automation), and
[imports](imports.md#limits-and-errors). These explain partial results, known
side effects, local persistence, and retry-safety metadata.

Messages are intended for humans. Automation must branch on `error.code` and
the process exit status. Stable error codes are:

| Code | Exit status |
| --- | --- |
| `invalid_input` | 2 |
| `configuration_error` | 3 |
| `insecure_transport` | 3 |
| `profile_not_found` | 3 |
| `profile_already_exists` | 3 |
| `profile_server_mismatch` | 3 |
| `profile_in_use` | 3 |
| `profile_has_credential` | 3 |
| `transport_error` | 4 |
| `unexpected_redirect` | 4 |
| `protocol_error` | 4 for protocol/response validation failures; 5 for a decoded HTTP-200 Wekan error without `statusCode`, or a native export text failure |
| `api_response_too_large` | 4 |
| `file_io_error` | 4 |
| `login_rejected` | 5 |
| `login_rate_limited` | 5 |
| `credential_not_found` | 5 when initially absent; 6 when a prepared mutation's credential disappears before execution |
| `credential_expired` | 5 |
| `credential_already_exists` | 3 |
| `authentication_rejected` | 5 |
| `permission_denied` | 5 |
| `not_found` | 5 |
| `registration_rejected` | 5 |
| `registration_disabled` | 5 |
| `server_error` | 5 |
| `batch_incomplete` | 5 |
| `credential_store_unavailable` | 6 |
| `credential_store_failed` | 6 |
| `internal_error` | 1 for unexpected internal failures, including initialization; 4 for output delivery |

Passwords, two-factor codes, and managed authentication tokens are redacted
from CLI-generated diagnostics. Commands that intentionally return an issued
secret document that output in their own topic. Exact raw API response headers
and bodies are upstream data and are not redacted.

## Exit statuses

| Exit | Meaning |
| ---: | --- |
| `0` | Success |
| `1` | Unexpected internal failure |
| `2` | CLI usage, missing/ambiguous identity, empty secret, or registration password mismatch |
| `3` | Missing or invalid server/profile configuration, profile conflicts, or insecure transport refusal |
| `4` | Transport, redirect, protocol/response validation, file I/O, or local response-output failure, including `api_response_too_large` |
| `5` | Unauthenticated state, authentication rejection, Wekan application/server rejection (including native protocol errors), or a partial batch/import result |
| `6` | Credential store unavailable or failed, a prepared mutation's credential changed/disappeared, or a created account/session whose token could not be stored |

When a non-success error response body is empty or is not JSON, its HTTP status
still determines the error classification and exit status. A syntactically
valid JSON error object with unmapped fields or invalid known-field types is a
`protocol_error`; it is never silently reduced to a generic server error.
