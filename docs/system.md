# Public system status

Read Wekan v11.06's live startup schema-upgrade state:

```console
wekan --server http://localhost:3000 system schema-upgrade-status
wekan --server http://localhost:3000 --output json system schema-upgrade-status
```

The command sends an anonymous `GET /schema-upgrade-status?json=1`. It supports
`--server <URL>`, `--profile <NAME>`, `--output human|json`,
`--allow-insecure-http`, and `--help`. These flags do not repeat.

A saved profile supplies its server. Before any profile or login exists,
`--server` or `WEKAN_URL` can supply the server without creating a profile,
changing the active selection, accessing credentials, or prompting. Existing
profile/server mismatch checks and remote HTTP restrictions still apply.
See [target selection](config.md#target-selection).

The normal success envelope contains `running`, `product`, `app_version`,
`started_at`, `finished_at`, `current_step`, `gated`, `steps`, and `last_check`.
The timestamps and previous check can be null. Populated dates are validated;
required keys must be present even when null.

`steps` is ordered by native step name. Each entry contains `status`, `fixed`,
`unresolved`, and optional `error` (null in CLI output when omitted by Wekan).
Native statuses are `pending`, `checking`, `running`, `done`, `skipped`, and
`error`. A populated `last_check` contains `version` and `at`.

A successful read exits zero even when the reported migration has failed steps
or unresolved records. Inspect the typed state to determine migration health.
Unknown fields at any object level, missing required fields, invalid types,
and invalid dates fail with `protocol_error`. Human output escapes terminal
controls and shows all mapped fields. Raw/HTML output and polling are unavailable
on this command; the raw API command can request the native HTML variant.
