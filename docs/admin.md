# Administrator commands

The `admin` family covers every administrator REST operation in Wekan v11.06.
All operations require a site administrator and use the selected profile's stored
credentials. Global settings and attachment-storage commands are described in
[administrator settings](admin-settings.md).

## Domains and resource listings

```console
wekan admin domain list
wekan admin org list
wekan admin team list
```

Domain rows contain `domain` and `count`. Wekan counts the first email address of
each user, lowercases and trims its domain, and skips addresses without a domain.
Rows retain Wekan's ordering: descending count, then alphabetically. Organization
and team rows contain their resource ID, `display_name`, `short_name`, and the
three boolean feature flags. Their order is preserved as returned by Wekan.

## Individual feature updates

```console
wekan admin org features update --org ORG_ID --shared-templates true
wekan admin team features update --team TEAM_ID --propagate-members-to-boards false --sync-members-from-auth true
```

Each command requires its ID selector and at least one of:

| Flag | Value |
| --- | --- |
| `--shared-templates` | `true` or `false` |
| `--propagate-members-to-boards` | `true` or `false` |
| `--sync-members-from-auth` | `true` or `false` |

Flags do not repeat. Omitted fields are preserved. Updates execute directly and
return the complete updated `organization` or `team`, plus `submitted_fields` in
the order shown above. These are native REST feature writes: the CLI does not
perform additional board-member propagation or authentication-provider sync.

## Instance-wide feature updates

```console
wekan admin org features update-all --field shared-templates --value true --yes
wekan admin team features update-all --field sync-members-from-auth --value false --yes
```

Both `--field` and `--value` are required. `--field` accepts `shared-templates`,
`propagate-members-to-boards`, or `sync-members-from-auth`. `--value` requires
explicit `true` or `false`. Each invocation changes one feature on every resource
of that kind, including inactive resources. These arguments do not repeat.

Human terminals prompt for confirmation unless `--yes` is supplied. Scripts and
JSON output require `--yes`. Declining returns the standard cancellation result
without an HTTP request. Successful output contains snake_case `field`, boolean
`value`, and Wekan's nonnegative `updated` count; zero is a successful result.

## Output and errors

All commands accept `--server URL`, `--profile PROFILE`, `--output human|json`,
`--allow-insecure-http`, and `--help`. JSON uses the standard success/error
envelope. Human output includes every mapped field and escapes terminal controls.
Resource `_id` fields become `organization_id` or `team_id`; business fields lose
the wire's `org`/`team` prefix and use snake_case. Optional values appear as null.
Full organization output includes tenant branding and theme fields; both resource
documents include creation and modification timestamps.

Responses are bounded to 64 MiB and decoded strictly. Unmapped fields, invalid
types, missing required fields, and invalid timestamps produce `protocol_error`.
The domain count and bulk update count must be nonnegative integers. Each command
sends one request, without a read-before-write merge or automatic retry.

Wekan v11.06 can return authorization errors inside HTTP 200 responses. The CLI
preserves both the HTTP status and embedded Wekan status/error metadata while
redacting stored tokens. Individual updates to absent IDs return empty HTTP 200
bodies: the CLI reports `protocol_error` without inventing a successful update
or a server 404.

Declining instance-wide organization or team updates returns the shared
cancellation result with operation `admin_organization_features_update_all` or
`admin_team_features_update_all`, respectively.
