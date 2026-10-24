# Board integrations and webhook subscriptions

Integration commands target Wekan v12.57. All require `--board BOARD_ID` and
accept `--server URL`, `--profile PROFILE`, `--output human|json`,
`--allow-insecure-http`, and `--help` (`-h`). Raw output is unsupported.

## Commands

```console
wekan integration list --board BOARD_ID
wekan integration get INTEGRATION_ID --board BOARD_ID
wekan integration create --board BOARD_ID --url https://hooks.example.com/wekan
wekan integration update INTEGRATION_ID --board BOARD_ID --enabled false
wekan integration update INTEGRATION_ID --board BOARD_ID --title "Delivery" --url https://hooks.example.com/delivery
wekan integration update INTEGRATION_ID --board BOARD_ID --token-stdin
wekan integration update INTEGRATION_ID --board BOARD_ID --clear-token
wekan integration update INTEGRATION_ID --board BOARD_ID --activity createCard --activity addComment
wekan integration update INTEGRATION_ID --board BOARD_ID --clear-activities
wekan integration activity add INTEGRATION_ID --board BOARD_ID --activity createCard --activity addComment
wekan integration activity remove INTEGRATION_ID --board BOARD_ID --activity all
wekan integration delete INTEGRATION_ID --board BOARD_ID --yes
```

Creation sends only the required URL. Wekan creates an enabled integration of
type `outgoing-webhooks`, subscribed to `["all"]`, without a title or token.
Use update to configure those fields. The URL is sent unchanged; Wekan validates
HTTP/HTTPS and rejects unsafe or unresolvable destinations.

Update accepts any combination of `--enabled true|false`, `--title TEXT`,
`--url URL`, `--token-stdin` or `--clear-token`, and repeated `--activity VALUE`
or `--clear-activities`. Omitted fields stay unchanged. An update without field
flags sends `{}` and receives an acknowledgement. An empty title clears it.
The integration type is read-only through these REST operations.

`--token-stdin` reads UTF-8 through EOF, removes one final LF or CRLF, and
preserves all other bytes. Empty input is rejected before an HTTP request;
`--clear-token` explicitly sends an empty string. The two flags conflict.
Tokens cannot be supplied as command arguments or environment variables.
They are redacted from diagnostics and debug output and excluded from all
integration read results.

## Activity subscriptions

Update's `--activity` flags replace the entire subscription array.
`--clear-activities` sends `[]` and conflicts with those flags.
Activity add and remove each require at least one `--activity`; flags may repeat.
The CLI preserves input order, duplicates, and activity strings without imposing
an enum. Wekan controls cleaning and storage.

Activity add uses Wekan's native set addition: already present values are not
added again, and duplicates already in the array remain. Remove deletes all
matching occurrences; absent values have no effect. Both return the saved array.
Removal executes directly, without `--yes` or a confirmation prompt.

Adding a named activity to `["all"]` leaves `all` present. To narrow subscriptions,
replace the array with update or remove `all` explicitly. `integration get`
shows the current subscriptions; there is no separate subscription GET endpoint.

## Results, permissions, and pinned behavior

List returns `board_id` and full `integrations` in server order. Get returns
`integration_id`, `board_id`, `user_id`, `enabled`, `title`, `integration_type`,
`url`, `activities`, `created_at`, and `modified_at`. An absent title is JSON null;
date strings are validated and preserved. Unmapped fields, invalid types, missing
required properties, and mismatched resource IDs produce `protocol_error`.

Create returns the board and new integration IDs. Update returns those IDs
and `submitted_fields`. Delete returns the board and integration IDs.
Activity mutations
return `board_id`, `integration_id`, `action` (`add` or `remove`),
`requested_activities`, and Wekan's returned `activities`. Human output renders
all mapped fields and escapes terminal controls.

Board access permits reads; mutations require board administration.
Integration deletion uses the existing confirmation flow, or `--yes` for scripts.
Declining confirmation returns a cancellation result without sending HTTP.

Wekan v12.57 returns HTTP 200 with an empty body for missing or wrongly scoped
integration lookups and activity mutations. The CLI reports `not_found` and
preserves `http_status: 200`. Update and delete return the requested
ID even when no integration matched.

Update writes enabled state, title, URL, token, and activities sequentially.
A later failure can leave earlier fields saved. For example, an invalid URL can
return HTTP 400 after the new title was written. Errors preserve server metadata
and expose the actual failure diagnostics. Commands perform
one native request without extra reads, emulation, or mutation retries.

Source evidence: pinned [REST routes](https://github.com/wekan/wekan/blob/v12.57/server/models/integrations.js)
and [integration schema](https://github.com/wekan/wekan/blob/v12.57/models/integrations.js).
The isolated lifecycle suite verifies these behaviors against the pinned Compose
image. Outbound webhook delivery is outside this slice.

Update and deletion report request acceptance because missing or wrongly scoped targets can produce successful no-ops.

Declining integration deletion returns the shared cancellation result with
operation `integration_delete`.

## Delivery overrides and metadata

`integration update` accepts `--notification-delivery JSON`, for example
`'{"webhook":{"text":false,"schedule":"daily","dailyTime":"09:30","timezone":"Africa/Nairobi"}}'`.
The typed webhook keys are `text`, `fields`, `grouping`, `schedule`,
`intervalMinutes`, `dailyTime`, `quietStart`, `quietEnd`, and `timezone`.
`--clear-notification-delivery` conflicts with the JSON input and restores
inheritance from board/administrator defaults. Wekan validates delivery settings
before any field writes. Reads expose `notification_delivery`,
`mattermost_direct_messages`, and `bot`; omitted metadata becomes null.

## Incoming email providers

```console
wekan integration inbound-email submit --file message.json --headers-file provider-headers.json
wekan integration inbound-email bounce mailgun --file bounce.json --headers-file provider-headers.json
```

These endpoints use provider authentication, without the selected profile's
session token. `--file PATH|-` preserves original bytes, including signed bodies.
`--headers-file PATH` is a regular JSON object of string header values; stdin is
reserved for the payload. Header values are secret inputs and are redacted from
diagnostics. `submit` and `bounce` accept `--content-type`, defaulting to
`application/json`; RFC822 and multipart messages require their native media
type and boundary.

Bounce providers are `mailgun`, `sendgrid`, `postmark`, `brevo`, `mailjet`,
`sparkpost`, `mandrill`, and `resend`. The server's configured provider secrets,
signatures and recipient matching determine acceptance.
Results are typed native receipts; the CLI does not fabricate signatures or
convert provider payloads.

The Amazon SES/SNS endpoint is intentionally excluded from typed CLI support.
