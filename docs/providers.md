# Provider integration status

The catalog contains fourteen agents. Catalog presence and a local icon do not imply that an account quota API is available. All displayed quota values come from a provider response, an authenticated managed-session bridge, or an explicitly configured local collector/file. No sample balances are shown in the normal UI.

## Built-in quota paths

- **Codex:** launches the installed `codex app-server`, reads account identity and `account/rateLimits/read`, then closes it. ChatGPT-backed login is required for subscription quotas; API-key-only login is reported separately. This read path does not start a model turn.
- **Claude Code:** reads an existing CLI OAuth credential file, or the CLI’s named macOS Keychain item, and queries the account usage service. The quota service is an internal OAuth endpoint and can reject scopes or rate-limit polling. Managed Claude sessions additionally deliver official status-line quota fields. Failed direct polling preserves a source-bound bridge snapshot as stale. Tokens are not refreshed by this application; renew login using Claude Code.
- **Gemini CLI:** reads an existing eligible Google OAuth login and the internal Code Assist quota response. Personal/free/AI Pro/Ultra accounts may require Antigravity under Google’s announced migration. Expired tokens or ineligible accounts are reported explicitly. This application does not implement an independent OAuth renewal flow.
- **GitHub Copilot:** reads compatible existing `apps.json`/`hosts.json` CLI credentials and quota snapshots from the account service. Newer or system-credential-store-only login formats may need a custom collector. Unlimited buckets are not represented as a fictitious percentage.

These adapters are implemented and parser/process-tested. They have **not** all been verified against the user’s live logged-in accounts. Account eligibility, CLI versions and internal service behavior can change.

## Catalog entries with collector integration

**Antigravity, Cursor, Windsurf, Cline, Roo Code, OpenCode, Continue, Kiro, Amazon Q Developer and Aider** have local entries, offline icons, executable/configuration discovery, account profiles and generic CLI sessions. Their native account quota APIs are not implemented in this version. Use the official usage-page action, a local quota JSON file, or a collector program configured in the account editor. An installed agent without a working collector is shown as “Collector required”.

The app currently detects CLI/configuration directories listed in `config/defaults.json`; it does not decrypt browser cookies or enumerate every IDE extension database. A desktop-only installation can be configured manually.

## Collector contract

A collector is an executable plus a JSON array of arguments; it is executed directly, without shell interpolation. For Node/Python collectors, choose the interpreter executable and pass the script filename as an argument. It receives `AGENT_ORBIT_PROFILE_ID`, `AGENT_ORBIT_PROVIDER`, and, when configured, `AGENT_ORBIT_PROFILE_DIR`. Output one JSON object on stdout:

```json
{
  "account": "account label or email",
  "plan": "plan label",
  "fetchedAt": 1792886400,
  "windows": [
    {
      "label": "5h",
      "windowMinutes": 300,
      "remainingPercent": 64,
      "used": null,
      "limit": null,
      "unit": "percent",
      "resetsAt": 1792904400
    }
  ]
}
```

The example is a schema illustration, not live quota data. Times are Unix seconds. `remainingPercent: null` means unknown; zero means exhausted. `windowMinutes` selects the shortest summary period, independently of reset countdown. Omit `fetchedAt` only when the collector fetched data just now. Additional fields such as tokens are discarded, but collectors should never output credentials. Output is bounded and requests time out according to settings. A local JSON file uses the same schema and takes precedence over a collector command.

## CLI interaction

- Every configured CLI executable can run in a managed PTY with keyboard input, resize and output replay. npm-generated Windows launchers are resolved to Node and their JS entry point; arbitrary batch files require explicit native/interpreter configuration.
- Codex offers a structured app-server session with command/file/permission approvals and user questions. Unknown server requests return an unsupported error; use terminal mode for interfaces that need the original terminal.
- Claude managed terminals receive process-local permission/question/status-line hooks. No global Claude settings file is modified. The managed status line takes precedence for that session. Originating account label, project directory and session ID appear on approval cards.
- Other agents’ prompts remain interactive in their terminal session. This version does not provide structured approval cards for every agent and cannot take over arbitrary terminals that were started outside the app.
- Editing a profile invalidates its saved quota. An old managed session remains bound to its original profile configuration and cannot write quota into the edited profile; restart the managed session to resume bridge quota updates.

## Icons

Eleven brand marks are bundled from the MIT-licensed Lobe Icons collection. Continue, Amazon Q and Aider use original lettermark placeholders. See [icon attribution](../public/icons/ATTRIBUTION.md). Users can change icon references through the external provider configuration; only packaged resources are currently available through the renderer asset policy.

For a read-only local integration check, `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --example probe -- codex` prints only provider status, source and window count. It does not print account email, quota values or credentials and does not start inference. Replace `codex` with another catalog ID as needed.
