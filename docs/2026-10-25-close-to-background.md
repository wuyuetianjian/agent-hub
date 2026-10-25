# Close the workspace and keep running

User requirement: closing the main workspace must leave the application running in the background, with quota/token monitoring, managed sessions and the floating HUD intact.

The existing native workspace CloseRequested event already prevents destruction and hides the workspace. Expose this behavior with an explicit titlebar close (×) button labelled “Close window and keep running”, and Ctrl/Cmd+W. Both use the caller-aware hide_window command. Keep “Quit application” as the separate explicit application-exit action. The existing taskbar/Dock/tray workspace reopening path remains available.

No additional confirmation, settings toggle or persistence change is needed for this requested close behavior. Validate TypeScript/production compilation and macOS app packaging, confirm the close action does not call quit_app or stop sessions, and verify archive integrity. Native desktop visual verification remains unavailable.

Preserve Ctrl+W inside text inputs and the interactive terminal for normal editing; Cmd+W retains the macOS window-close convention.

Validation completed: TypeScript/Vite and native macOS app build passed; Rust formatting and git whitespace checks passed. Source inspection confirms both workspace hide paths bypass quit_app and session termination.
