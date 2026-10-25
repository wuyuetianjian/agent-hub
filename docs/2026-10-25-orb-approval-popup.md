# Circle approval popup

The approved interaction requires real pending requests to open the corresponding AI's independent detail window beside its circle automatically, including while the workspace is hidden. The request must remain actionable until resolved or explicitly dismissed. Hover and click must still open details. Resolving an approval must clear its badge and close an unattended, unpinned popup. Imported read-only histories remain excluded.

Native testing reproduced a badge without a visible confirmation panel. Investigate the dock-to-native show path before changing it; preserve taskbar isolation and avoid activating the workspace. Regression checks cover automatic selection with requests arriving before session metadata and native showing, followed by a real harmless Codex request.

Native ownership: dashboard events announce each new live request once, show the independent panel, and synchronize the rail selection. Repeated refreshes do not reopen a manually dismissed request. Clicking a circle permits keyboard focus; hover and automatic opens preserve current focus. Session metadata updates also retry frontend selection.

Native validation found raw exec-policy JSON overlapping adjacent action buttons. Include readable localized labels for command-rule and session approvals and Cancel, preserve exact wire decisions, and wrap unknown labels safely.

## Validation

Rust: 70 tests pass, including native announcement ordering/deduplication and subprocess decision-label regression coverage. Frontend: 28 tests pass; TypeScript and Vite production build pass. macOS debug application bundle and v0.1.9 distribution ZIP built. Real Codex test in a disposable directory produced a live approval while the workspace was hidden. The independent circle detail panel displayed the request; clicking Allow once there resolved it, and the managed session returned `ORBIT_CIRCLE_CONFIRMED`. The desktop tool selects the key window: it captured the circle rail for a non-activating automatic open, and the separate panel after clicking the circle to focus it. Therefore the native automatic show path is implemented but the no-click visual appearance was not independently captured. Windows/Linux native UI testing was not performed on this macOS machine.
