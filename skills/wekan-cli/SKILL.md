---
name: wekan-cli
description: Operate Wekan through the wekan CLI. Use for Wekan board and task workflows or questions about CLI commands, authentication, JSON output, and errors.
license: Apache-2.0
---

# Wekan CLI

Use the installed `wekan` executable for Wekan tasks. Before operating it, load
the guide bundled with that version:

```console
wekan skill show
```

Read additional topics only as the task requires:

```console
wekan skill list
wekan skill show cards
```

Use `wekan <command> --help` for exact arguments and options. Detailed guidance
comes from the installed CLI, so it follows CLI upgrades without a separate
copy of the command manual in this skill.

If `wekan` is unavailable, report that it must be installed or made available
on PATH before continuing. If it lacks `skill show`, report that this discovery
skill requires a version providing embedded guidance. Do not automatically
install or upgrade software. Preserve the user's chosen version and target.
