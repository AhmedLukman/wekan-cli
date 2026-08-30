# Security Policy

## Supported versions

Wekan CLI is pre-release software and has no published versions yet. Security
fixes are made only on the current `main` branch; fixes are not backported to
earlier commits or snapshots.

| Version | Supported |
| --- | --- |
| Current `main` branch | Yes |
| Earlier commits and snapshots | No |

This table will be updated when the project begins publishing releases.

## Reporting a vulnerability

Please report suspected vulnerabilities privately through the
[GitHub security advisory form][private-report]. Do not disclose an unresolved
vulnerability in a public issue, discussion, or pull request.

Include as much of the following information as possible:

- A description of the vulnerability and its potential impact.
- The affected Wekan CLI commit, Wekan server version, and operating system or
  platform.
- Reproduction steps or a minimal proof of concept.
- Relevant logs or output, with credentials and personal data removed.
- A suggested remediation, if you have one.

Never include passwords, access tokens, API keys, real Wekan data, or other
secrets in a report. Use synthetic test data and redact sensitive output.

We aim to acknowledge complete, actionable reports within 48 hours. The
maintainers will then validate the report, assess its severity, and coordinate
remediation and disclosure with the reporter. Remediation time depends on the
complexity and impact of the issue, so no fixed resolution deadline is
promised.

## Scope

Report vulnerabilities caused by Wekan CLI here, such as credential exposure,
unsafe HTTP behavior, or unintended destructive mutations. Report Wekan server
or REST API vulnerabilities through the [upstream security policy][wekan-security],
and non-security CLI bugs through the [public issue tracker][issues].

[private-report]: https://github.com/AhmedLukman/wekan-cli/security/advisories/new
[wekan-security]: https://github.com/wekan/wekan/security/policy
[issues]: https://github.com/AhmedLukman/wekan-cli/issues/new
