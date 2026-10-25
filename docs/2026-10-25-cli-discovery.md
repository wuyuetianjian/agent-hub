# CLI discovery from desktop applications

## Problem

A GUI launched from Finder may have a minimal PATH. The original discovery code only used `which`, so an npm-installed Codex under NVM could be reported as missing. Finding its script is not sufficient: `#!/usr/bin/env node` can still fail when the GUI PATH has no Node.

## Required behavior

- Explicit profile executable remains the highest-priority choice and is never silently replaced when invalid.
- Otherwise search inherited PATH, then configurable executable-path patterns from the provider catalog. Cover common NVM, Volta, FNM, user-local, Homebrew/npm and Codex desktop bundle layouts.
- Recognize a desktop installation separately from a usable CLI; do not call a detected desktop-only installation “not installed”. Only actual CLI candidates may be executed.
- Resolve Node shebang launchers through a sibling Node executable when available, without sourcing shell startup files or invoking a shell.
- Keep candidate paths in external configuration, with no developer-specific username or pinned Node version in source.
- Test discovery with an empty/minimal PATH, multiple NVM versions, explicit overrides and desktop-only installations. Test Node launcher arguments and the detected local Codex without invoking inference.

## Validation

Run all existing tests, add regression tests, rebuild the macOS development bundle and update the release archive. Live quota lookup may still be blocked by protected Codex state in this sandbox; executable discovery can be independently verified.

## Existing sessions

Discover Codex CLI/desktop session metadata from the account’s read-only thread index, with JSONL session metadata fallback. Existing records must appear separately from managed live sessions; history metadata does not prove that a process is active. Display title, project, source and last update. Do not send terminal input or approval replies into external records without a supported live connection. The session list is bounded and does not read full conversation bodies.

## Result

The minimal-PATH local diagnostic detected the NVM Codex launcher, selected its sibling Node, detected the bundled desktop CLI, and read 50 existing session records (the configured default limit). Regression coverage includes archived-only indexes, literal bracketed directories, inherited account roots and first-line-only fallback. Existing-session scanning runs outside the native UI thread and is cached. All changes are packaged as v0.1.1.
