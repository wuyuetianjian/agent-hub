# Edge dock interaction refinement

Approved direction: stable per-orb hover magnification; separate dock, detail popup and workspace windows; hover summaries with click-to-pin; real drag-release snapping and position locking; clearer size labels/presets and live preview. References: CodexBar, Claude Usage Tracker, Token Monitor and Codenotch, researched 2026-10-25.

Additional user requirements: resting orbs contain only the provider icon and remaining-quota perimeter. Percentage text must appear only on hover/focus in the detail popup, never overlap the icon or ring. Dock should sit as close to the physical display edge as practical. OS taskbar/Dock activation must open/focus only the workspace; floating windows must not have independent taskbar entries.

Use fixed-size orb cells based on the configured maximum size. Magnify only the pointer/focus target within its cell; other orbs and native dock geometry remain stationary. Default edge inset is zero and edge-facing padding is removed. Existing previous default inset migrates to zero, while custom values remain.

The detail popup is a separate taskbar-excluded window, positioned inward from the selected orb and clamped to its monitor. Hover does not focus or restore a minimized workspace. Click pins a summary; close/Escape unpins/dismisses it. Pending confirmations retain originating account/project context, request ID and drafts. Dismissal hides the popup but retains pending badges. Moving between dock and popup preserves the popup.

Dragging uses pointer capture and explicit begin/move/end commands. Only pointer release snaps to the nearest edge. A position-lock setting prevents dragging. Geometry is persisted by side; reset position recenters the selected side. Settings changes apply immediately, with small/medium/large presets, explicit resting/hover labels and preview. Short-window quota stays primary; exhausted secondary windows get a distinct alert marker. Unknown, missing login, missing installation and collection failures remain distinguishable.

Taskbar association: dock/popup skip taskbar and workspace is the normal application window. macOS Reopen, tray left click and tray Show target the workspace. Workspace close/minimize does not hide, move or resize floating windows.

Validation: regression tests for resting markup with no percentage badge, detached panel content, source/profile-safe approvals, edge geometry and migration, persisted lock/settings. Run frontend/Rust tests, production/native builds and package integrity checks. Native visual and OS taskbar behavior require verification on a real desktop; current automation has no working native UI surface.

## Validation and review

All 49 Rust tests and 19 frontend tests passed. TypeScript/Vite production build, native macOS app bundling, formatting and whitespace checks passed. Review corrections preserve the actual displayed side when saving geometry, reposition an already open panel after live settings changes, and clear detached panels when switching to island mode. Popups temporarily disable focusability while first showing so hover does not deliberately activate the user's editor away. OS-level behavior remains subject to manual desktop verification.

Reference sources: https://github.com/steipete/CodexBar, https://github.com/hamed-elfayome/Claude-Usage-Tracker, https://github.com/Javis603/token-monitor, https://github.com/vinzdg/codenotch.
