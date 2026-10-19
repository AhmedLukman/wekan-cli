# Agent skills and embedded guidance

Wekan CLI supplies a portable discovery skill and a guide bundled with the
executable. The discovery skill tells an agent where to retrieve instructions;
the detailed instructions come from the installed CLI version. Guide retrieval
is offline and needs no source checkout, server, profile, or credential vault.

Installing the skill and installing Wekan are separate actions. Install the
CLI using the [README](../README.md#install), then install the discovery skill
using one of the methods below. Neither the skill nor the CLI installs or
upgrades the other automatically. Restart or refresh the agent if a newly
installed skill does not appear.

## Embedded guide commands

```console
wekan skill list
wekan skill show
wekan skill show cards
wekan --output json skill show agent-contract
```

Complete syntax, including all inherited global options:

```text
wekan [--server <SERVER>] [--profile <PROFILE_NAME>] [--output human|json] [--allow-insecure-http] skill list
wekan [--server <SERVER>] [--profile <PROFILE_NAME>] [--output human|json] [--allow-insecure-http] skill show [TOPIC]
```

`--help` is available at every command depth. `skill list` lists topic names and
descriptions alphabetically. `skill show` defaults to `core`; topic names are
exact and case-sensitive. Other operational documents use their filename stems,
such as `auth`, `cards`, and `agent-contract`. The core guide explains how to
follow operational Markdown links through topic commands. Command-specific
`--help` supplies available commands, arguments, and flags; bundled topics add
workflow and behavioral guidance. This installation guide, the complete command
inventory, and development, architecture, audit, and API-coverage documents
remain repository references and are not bundled topics.

Human `show` output is the document's Markdown with one final newline. Human
`list` output is a topic/description table. JSON mode follows the standard
[agent contract](agent-contract.md); success `data` is:

```json
{"cli_version":"1.0.0","topics":[{"name":"core","description":"Start here: agent workflows, discovery, and essential conventions"}]}
```

The example above abbreviates the list. `show` instead returns:

```json
{"cli_version":"1.0.0","topic":"core","content":"# Using Wekan CLI from an agent\n..."}
```

`cli_version` is the executable's package version. `content` is the complete
embedded document, retaining its source line endings. Successful envelopes go
to stdout; errors go to stderr. Unknown topics return `invalid_input`, exit 2,
with guidance to run `skill list`. `--output raw` is unsupported and returns
the existing usage error before configuration access. Help remains text even
with JSON selected.

These commands do not resolve a server or profile. Target-related global flags
are accepted but have no effect on guidance. Invalid inherited configuration
and unavailable credentials do not prevent reading it. Normal parser validation
still applies, including profile-name syntax. Nothing is written to disk and
stdin is not read.

## Install using the Skills CLI

The [Skills CLI](https://github.com/vercel-labs/skills) downloads the canonical
`skills/wekan-cli` folder from this repository. It requires Node.js/npm. Install
for the current project or for all projects belonging to the current user:

```console
npx skills add AhmedLukman/wekan-cli --skill wekan-cli --agent codex
npx skills add AhmedLukman/wekan-cli --skill wekan-cli --agent claude-code
npx skills add AhmedLukman/wekan-cli --skill wekan-cli --agent codex --global
npx skills add AhmedLukman/wekan-cli --skill wekan-cli --agent claude-code --global
```

Choose the command for the desired agent and scope. Add `--copy` if symlinks
are unavailable. Skills remain hosted in this repository; skills.sh can list
them through the installer's anonymous installation telemetry. A separate
registry upload is unnecessary.

Update the discovery skill in its original scope:

```console
npx skills update wekan-cli --project
npx skills update wekan-cli --global
```

Remove a project install with `npx skills remove wekan-cli --agent codex` or
`npx skills remove wekan-cli --agent claude-code`. Add `--global` to remove a
user install. Use the installer to remove its managed copies or links.

## Install using GitHub CLI

GitHub CLI's [skill commands](https://cli.github.com/manual/gh_skill) support
the same repository folder without Node.js. They are currently a preview
feature; use a `gh` version that exposes `gh skill`:

```console
gh skill install AhmedLukman/wekan-cli wekan-cli --agent codex --scope project
gh skill install AhmedLukman/wekan-cli wekan-cli --agent claude-code --scope project
gh skill install AhmedLukman/wekan-cli wekan-cli --agent codex --scope user
gh skill install AhmedLukman/wekan-cli wekan-cli --agent claude-code --scope user
```

Use `--pin` with a tag or commit containing the skill for a reproducible
installation. Run `gh skill list` to inspect installed skills and
`gh skill update wekan-cli` to update an unpinned install. To move a pinned
install to the latest source, use `gh skill update wekan-cli --unpin`.
The current command group has no removal subcommand: remove only the installed
`wekan-cli` directory reported by `gh skill list`, or use the host agent's
skill-management UI where available. Do not remove other skills or agent
configuration.

## Install manually

Copy the complete `skills/wekan-cli` folder from a trusted checkout or release
source into the desired agent directory. The destination must contain
`wekan-cli/SKILL.md`:

| Agent | Current-project destination | Current-user destination |
| --- | --- | --- |
| Codex | `.agents/skills/wekan-cli/` | `~/.agents/skills/wekan-cli/` |
| Claude Code | `.claude/skills/wekan-cli/` | `~/.claude/skills/wekan-cli/` |

`~` means the home directory, including the Windows user profile. No symlinks
or installer dependencies are required. For another compatible agent, use its
documented skill discovery directory. Update a manual install by replacing
that folder from the desired source revision; remove it by deleting that exact
folder. Avoid installing duplicates through several methods in the same scope.

## Updates and maintenance

Upgrade Wekan through its normal installation method to update the embedded
guide and command help. The small discovery skill needs a separate update
only when its loading instructions or discovery metadata change. An older CLI
that does not expose `skill show` cannot serve this guide; the skill reports
that requirement rather than upgrading it automatically.

The maintained Markdown sources are compiled directly into the executable.
The crate contains those source documents so `cargo install` builds the same
guidance as release binaries. There is no runtime download or external guide
directory override. Version alignment does not replace maintaining accurate
documentation when behavior changes. The source skill uses the shared
SKILL.md format without host-specific tool permissions, scripts, or plugins.
