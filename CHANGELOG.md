# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Profiles for connecting to different Wekan servers, with credentials stored
  securely by the operating system.
- Commands to register, log in, check authentication status, and log out,
  including two-factor authentication support.
- Commands to view and manage Wekan users.
- Commands to view and manage boards, lists, swimlanes, cards, and comments.
- Human-readable and JSON output.
- `wekan api request` for API operations without dedicated commands, including
  raw response output.
- Interactive confirmation prompts for destructive commands, with a `--yes`
  flag to skip the prompt.
- Strict response validation against the supported Wekan API contract.
