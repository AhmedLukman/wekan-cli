# Administrator commands

The `admin` family covers every administrator REST operation in Wekan v12.57.
All operations require a site administrator and use the selected profile's stored
credentials. Global settings and attachment-storage commands are described in
[administrator settings](admin-settings.md).

## OAuth providers and passwordless sign-in

```console
wekan admin oauth-provider get
wekan admin oauth-provider update google --enabled true --id CLIENT_ID --secret-file oauth-secret.txt
wekan admin passwordless update --enabled true
```

Provider keys are `google`, `github`, `facebook`, `twitter`, `meteor-developer`,
`weibo`, and `meetup`. Updates require both `--enabled true|false` and `--id`;
an empty ID clears it. `--secret-file PATH` and `--secret-stdin` conflict.
Omitting the secret or supplying an empty secret preserves the stored value.
Optional `--login-style popup|redirect|""` changes the provider override;
`--global-login-style` changes the shared default, and
`--merge-existing-users true|false` controls account merging.

Results report configuration sources and secret-presence flags. An unset
configuration source may omit its native value; the CLI represents this as
null. Passwordless results report the source and value for email-code sign-in.
Secret values are excluded from output and diagnostics.

## Problem reports

```console
wekan admin problem list
wekan admin problem get security --limit 50 --skip 0 --search export
wekan admin problem acknowledge security
```

Problem streams are `security`, `speed`, `tests`, `cpu`, `ram`, `disk`,
`database`, `integrity`, and `mail`; `get` additionally accepts read-only `api`
usage. Pagination uses `--limit 1..200` and nonnegative `--skip`, with optional
case-insensitive `--search`. Acknowledgment marks existing problems as seen.
Results preserve typed overview entries, stream counts, event rows, and native
acknowledgment results.

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

Wekan v12.57 returns native HTTP 401/403 for administrator authorization
failures. The CLI retains status and error metadata while redacting stored
tokens. Individual updates to absent IDs still return empty HTTP 200
bodies: the CLI reports `protocol_error` without inventing a successful update
or a server 404.

Declining instance-wide organization or team updates returns the shared
cancellation result with operation `admin_organization_features_update_all` or
`admin_team_features_update_all`, respectively.
