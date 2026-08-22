# Wekan CLI

This is an agent-first, human-friendly, cross-platform Rust CLI for Wekan. Prefer deterministic commands, stable structured output, explicit errors and safe mutations when destructive. Write clean, organized code that fully satisfies the goal while avoiding unnecessary complexity and speculative functionality.

## Repository folder structure

Before modifying code in this repository, read and follow the
[repository folder structure guide](docs/folder-structure.md). Use it as the baseline,
not a fixed or exhaustive structure. It is a growth map, not a requirement to
create every directory immediately. Add folders when a concrete, documented
ownership or tooling need justifies them; do not create empty placeholders.

## Commit messages

Start each commit message with the appropriate Conventional Commits prefix,
such as `docs:`, `feat:`, `fix:`, or `chore:` etc. Follow the prefix with a
single-sentence summary that clearly describes the changes in the
commit as a whole; avoid vague descriptions.

## Local Wekan stack

The root [`compose.yaml`](compose.yaml) provides a reproducible Wekan `v11.06`
environment for local development and integration testing. It starts Wekan with
its REST API enabled and a MongoDB single-node replica set, exposing Wekan only
on localhost on `WEKAN_PORT` (default `3000`).

## Wekan API reference

The pinned baseline is Wekan `v11.06`. Always use references from the exact Wekan version being supported.

- Upstream endpoint catalog: [vendored Swagger 2.0 specification](spec/wekan.yml)
- Upstream human-readable reference: [vendored Redoc HTML](spec/wekan.html)
- Verified corrections: [OpenAPI Overlay 1.1](spec/wekan.overlay.yml)
- Combined contract: [generated corrected specification](spec/wekan.corrected.yml)
- Human-readable corrected reference: [generated Redoc HTML](spec/wekan.corrected.html)
- Authoritative implementation and tests: [Wekan v11.06 source](https://github.com/wekan/wekan/tree/v11.06)

Use the vendored `wekan.yml` for endpoint discovery and initial request/response shapes. Treat `wekan.yml` and `wekan.html` as read-only upstream material and record only verified correction deltas in `wekan.overlay.yml`. Never edit `wekan.corrected.yml` or `wekan.corrected.html` directly; regenerate them from the base and overlay.

Generate and validate the corrected machine-readable contract with the
Speakeasy `openapi` CLI:

```console
openapi overlay validate spec/wekan.overlay.yml
openapi overlay apply --schema spec/wekan.yml --overlay spec/wekan.overlay.yml --out spec/wekan.corrected.yml
openapi swagger validate spec/wekan.corrected.yml
```

Generate the corrected human-readable reference from the corrected contract
with the `redocly` CLI:

```console
npx --yes @redocly/cli@2.46.0 build-docs spec/wekan.corrected.yml --output spec/wekan.corrected.html --title "WeKan REST API v11.06 (Corrected)"
```

Regenerating the API reference artifacts requires Speakeasy OpenAPI CLI
on PATH and Node.js/npm for the pinned Redocly invocation.

## Inconsistencies and verification

Wekan's REST API documentation might be incomplete and generated annotations can be stale or wrong. Content types, request bodies, permissions, status codes, response shapes, and even documented operations may differ from real server behavior.

For every new or changed API-facing command:

1. Implement the matching-version specification first.
2. Verify it with an integration test against that Wekan version.
3. If it fails or behaves differently, do not guess or hide the mismatch with a generic workaround. Determine the intended behavior from the matching Wekan route implementation, Wekan API tests, docs, release notes etc.
4. Implement the verified behavior and add a regression test that records it.
5. If the verified behavior differs from `wekan.corrected.yml`, record the correction and its matching-version evidence in `wekan.overlay.yml`, then run the generation and validation commands above to regenerate `wekan.corrected.yml` and `wekan.corrected.html`.
6. If intent remains uncertain, report the uncertainty.

Observed server behavior determines compatibility, but document behaviour believed to be an upstream defect.
