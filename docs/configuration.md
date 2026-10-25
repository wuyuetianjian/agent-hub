# Configuration

User-editable settings and account profiles live in SQLite (`orbit.db`) in the platform’s local application-data directory under `agent-orbit`. Window positions are also persisted there. Credentials remain with the original CLI or system store.

Precedence is database settings, process environment, external configuration file, then bundled defaults. Existing database settings therefore take precedence over a changed default or environment refresh interval.

- `AGENT_ORBIT_DATA_DIR`: choose the application data directory, including a mountable location.
- `AGENT_ORBIT_CONFIG`: path to an external JSON file, recursively merged with [defaults](../config/defaults.json). Provider arrays are replaced as a whole.
- `AGENT_ORBIT_REFRESH_SECONDS`: process-level refresh default, between 15 and 86400 seconds. It does not replace an already saved database setting.
- `AGENT_ORBIT_STARTUP_TRACE`: optional writable file for native startup phase diagnostics; contains phase labels, not credentials.
- `CARGO_HOME`: build-tool cache only. The supplied npm desktop scripts default it to `.cache/cargo` when unset.

Example external configuration:

```json
{
  "settings": {
    "locale": "zh",
    "refreshSeconds": 120,
    "windowMode": "floating",
    "summaryPolicy": "shortest"
  }
}
```

macOS users can choose `windowMode: "island"` in Settings. This is an app-drawn top-of-screen layout, usable on displays with or without a notch. Click an icon to open that account’s details; the expanded workspace remains available. The default `shortest` summary picks a five-hour bucket before a weekly bucket. The alternative `lowest` summary picks the smallest known remaining percentage. Neither policy treats an unknown percentage as zero.

Edit a provider’s profile to configure executable, arguments, credential/configuration directory, quota file or collector. Argument fields are JSON arrays of strings. Multiple profiles can be created for the same provider. Explicit profile directories are important when monitoring multiple logged-in accounts.

Built-in credentials are bound to their original HTTPS service origin. External endpoint configuration can alter a path on that origin, but changing the host or port is rejected rather than sending existing credentials to another service. Use a custom collector when integrating a different service.

Linux window positioning and always-on-top behavior depend on X11/Wayland and the compositor. If a tray cannot be created, Hide returns an error and keeps the window accessible. Window positions are restored within current monitor bounds; unavailable monitor positions are moved onto a current display.

No remote control endpoint is exposed. Managed Claude hooks use an ephemeral IPv4 loopback listener and a separate random token per session. Requests with browser Origin headers, invalid tokens, exited sessions or changed quota bindings are rejected.

Existing Codex session metadata is refreshed in a background worker and cached. `externalSessionLimit` defaults to 50 records per profile; `externalSessionRefreshSeconds` defaults to 30 seconds. Both can be overridden through the external settings file before database settings take precedence. Explicit account directories override inherited `CODEX_HOME`; default account directories are used only when neither is set. Large JSONL archives are scanned with a bounded entry budget.

Provider catalog entries may define `executablePaths`, `desktopPaths`, `configEnv`, `sessionIndexPaths` and `sessionLogPaths`. Executable and desktop patterns support home-directory and environment-variable expansion. Session path patterns are relative to the effective account root, which is treated literally even if it contains brackets.

`dockPointerPollMs` (default 80, range 30–1000 milliseconds) controls native cursor sampling for inactive floating windows. Database settings override the mountable config. Standard webview hover remains a fallback when global cursor location is unavailable (for example, some Wayland compositors). `dockHoverDelayMs` remains the separate delay before closing after leaving both the circle and its detail panel.

Floating account details include a profile-scoped token timeline. `get_usage` accepts an optional `profileId`; omitting it retains the main dashboard aggregate. Profile-specific reports never combine another account's events.
