# Validation record

Implementation checked locally on macOS with Node.js 24.20.0, Rust 1.92.0, Tauri CLI 2.12.4 and Tauri 2.12.5. The build target is x86_64-apple-darwin.

## Automated checks

- Core tests cover quota zero/missing/invalid values; Codex/Claude/Google parsing; SQLite precedence/reopening; clearing and atomically rejecting snapshots after account-profile edits; duplicate, cancelled, expired and cross-session confirmations.
- Real local subprocess tests cover JSON-RPC correlation/disconnect; cleanup when an RPC owner is dropped; blocked stdin deadlines; PTY Unicode input/output; immutable session-profile bindings; Windows npm launcher resolution without a shell; collector credential-field filtering.
- Window geometry tests cover off-screen monitor recovery, preserving visible saved positions and fitting oversized windows.
- Frontend tests cover shortest-period/lowest-remaining summaries, unknown and zero quotas, elapsed reset wording and account/project context on approval cards.
- TypeScript/Vite production build and native macOS debug build are run locally. The desktop bundle command is `npm run desktop:build -- --debug --bundles app`.

Latest full run: 42 Rust tests and 6 frontend tests passed. TypeScript/Vite production build, Rust formatting and macOS app bundling passed.

## Independent review

A read-only independent code review identified account-binding writes from old Claude sessions, missing context on approval cards, RPC cleanup and write-deadline issues, Windows npm launcher handling, failed-poll replacement of bridge quota, and hiding without a tray. Fixes are included with targeted tests where the behavior is testable without a desktop. Native tray availability now guards Hide; restore checks current monitor bounds.

## Live integration limits

A read-only Codex quota probe was attempted without starting inference. The installed app-server exited because the current sandbox permits reading `/Users/sunny/.codex` but prevents initializing its SQLite state there. Therefore no live Codex account quota was verified in this environment. Credentials were not copied elsewhere to work around the restriction.

Other built-in provider quota parsers are tested with synthetic data; full live-account verification is outstanding. This is especially relevant to internal OAuth services and Google account eligibility. No paid model calls, purchases, entitlement resets or new logins were performed for validation.

A native startup smoke test with account polling disabled aborted with signal 6 before a window could be observed. LLDB could not launch the process (`lost connection`), and the environment could not list native processes (`sysmond service not found`). A minimal Tauri program without this app’s business code did not reach setup during the observation window and reported invalid connections to macOS Launch Services / `com.apple.hiservices-xpcservice`. Native startup is therefore not verified; this remains an explicit release blocker until reproduced and resolved on a usable desktop session.

The browser/native UI automation inventory was unavailable (`Browser is not available: iab`; native pipe startup failed), so screenshot-based layout validation and interactive native behavior have not been observed. Source/build checks do not replace that check.

## Other platforms

The GitHub Actions workflow builds and tests native Windows, Linux and macOS targets. Those jobs have not been executed in this local session. Windows/Linux installer creation, actual runtime behavior, Wayland compositor behavior and Mac notch hardware placement remain to be verified on their corresponding systems. The locally generated app is a development bundle, not a signed/notarized public release.

## v0.1.1 installation and existing-session fixes

A minimal GUI-style PATH (`/usr/bin:/bin`) was used for read-only local discovery. It detected the NVM-installed Codex CLI, the sibling Node executable and the desktop-bundled CLI installation, and imported 50 existing Codex session records (the default limit). No inference or quota request was required. Tests cover native discovery fallbacks, Node shebang launch, existing CLI/desktop metadata, archived indexes, explicit/inherited account directories and literal bracketed paths. Existing metadata is marked read-only, not inferred to be an active process.

The full suite now passes 42 Rust tests and 6 frontend tests locally. The Windows-specific runtime and prior native UI automation limitations remain unchanged.
