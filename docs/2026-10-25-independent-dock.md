# Independent responsive desktop dock

The floating widget and workspace/settings use separate native windows. Opening, closing, moving or resizing the workspace must not change the dock. Shared quota/settings updates continue through the existing event bus.

Add live size sliders for resting (default 32px) and hovered (default 44px) orbs. Settings persist immediately and update the dock without a save click. Hover/focus expands the rail and details; leaving shrinks it after the configured delay. Pending confirmation remains available at its originating orb. Icons scale with the orb, unknown quota remains unknown.

Dragging the dock handle snaps the rail to the closest left/right edge of the current monitor after movement settles, preserving/clamping its vertical position. The selected side is persisted. Programmatic resize must not trigger drag snapping.

The workspace is a distinct window with its own saved geometry, opened on the requested settings/agent/session page. Closing it hides only that window. The widget remains visible. Platform APIs must use the caller window where relevant.

Validation: geometry regression tests for edge choice and vertical retention; settings persistence and range validation; frontend resting/hover sizes; full tests and native app build. Native visual QA remains limited by the available desktop automation environment.

Existing v0.1.2 settings without a hover-size field migrate the previous 60px default to the configured resting default. Customized sizes are retained and hover size is raised to at least the resting size. Other preferences are preserved.

## Verification results

All 48 Rust tests and 14 frontend tests passed, including legacy settings migration, custom size preservation, invalid size rejection and offset-monitor edge selection. TypeScript/Vite production build, Rust native build, formatting checks and macOS v0.1.3 ZIP integrity passed. Independent code review found cross-window session/launch routing defects; both were corrected by passing the originating session ID into workspace navigation. Native visual behavior cannot be inspected in the current desktop automation environment.
