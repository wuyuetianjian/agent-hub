# Interactive approval diagnosis and regression tests

User report: a real running session did not produce an interactive confirmation popup. Native UI access was refused by the desktop tool, so do not claim a visual end-to-end pass. Request the source Agent and whether the session is managed or external while independently testing protocol routing.

Confirmed code defects: LaunchDialog always defaults to terminal mode (also when switching to Codex), which has no structured Codex approval bridge; structured thread/start inherits the CLI's approval policy, potentially never requesting permission. Existing imported sessions are read-only history and cannot be silently attached to their original client's stdin/approval UI.

Fix: use provider-configured defaultSessionMode (Codex structured, others terminal), preserve explicit terminal selection, and set the managed Codex thread approvalPolicy from database-backed settings (default untrusted). Never change the user's global CLI configuration or weaken the existing sandbox. Explain terminal-vs-floating confirmation behavior in launch UI. Publish actual interactionChannel metadata on managed/imported session records and show connection capability in the provider details.

Tests use a harmless Node fake app-server subprocess with the real Sessions/RpcClient/Broker chain. Cover allow, deny, free-text answers, concurrent sessions sharing wire IDs, upstream cancellation, timeout, and configured thread policy. No real model requests or shell commands are executed. Broker tests additionally cover duplicate clicks, expired requests and invalid/incomplete answers. Native popup clicking remains unverified until desktop access is available.

## Validation results

68 Rust tests and 27 frontend tests passed. The five new subprocess integration scenarios passed: allow/deny with same wire ID in concurrent sessions, Unicode free-text replies, explicit startup policy, timeout decline, and upstream resolution without a stale reply. Local installed Codex JSON Schema confirms thread/start.approvalPolicy accepts `untrusted` and `on-request`; official documentation was checked at https://developers.openai.com/codex/app-server. Read-only code review found no important regression.

Desktop inventory found the running app, but the computer-use tool refused access (“Computer Use was not approved to use Agent Orbit”). No actual native-window clicks were performed, and no visual end-to-end success is claimed.
