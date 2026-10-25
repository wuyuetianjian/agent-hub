# Agent HUD reference alignment

Reference requested by the user: https://github.com/jazzenchen/agent-hud-open, especially README Simple mode, AgentPeek.tsx, HUDMarkStrip.tsx and the c06 parity notes (read 2026-10-25). Implement original components inspired by its information hierarchy; do not copy third-party components, artwork or bot sprites.

Keep previous requirements: AI logos inside remaining-quota rings, no permanent percentage labels, physical-edge placement, independently managed floating and workspace windows, taskbar activation only for workspace.

Adopt three presentation levels: quiet compact dark edge rail; hover peek with the account and concise per-window remaining percentages/reset times; click-to-pin full quota/session details. Pending approvals/questions bypass the peek and open their existing actionable card beside the originating AI. Leaving an unpinned peek closes it after the configured delay. Opening a full workspace/session dismisses only the detail popup, leaving the edge rail intact.

Expose real session status: pending confirmations first, managed running sessions next, otherwise idle. Imported read-only session history must not imply live agent activity. Use a restrained live-session indicator; do not invent model thinking/tool activity or historical token/cost charts unsupported by the collected data. Respect reduced motion.

Validation: regression tests for compact hover content vs pinned details, source-safe activity status, no resting percentage badges and pending approvals; full frontend/Rust tests, production/native build and macOS archive validation. Native visual inspection remains unavailable in this environment.

## Verification results

49 Rust tests and 21 frontend tests passed. TypeScript/Vite build, macOS native app build and formatting/whitespace checks passed. Review found that the browser preview initially omitted compact mode and exact-session callbacks; both were aligned with the desktop path before packaging.
