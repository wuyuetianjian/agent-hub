# Side orb dock redesign

## User-requested behavior

- Replace the default card-style widget with a narrow vertical row of circular agent buttons at a screen edge.
- Put the provider icon in the center and represent remaining quota with a circular perimeter. Keep the shortest-window summary policy (e.g. 5h). Unknown quota must have an unknown/dashed ring, not a fictitious empty balance.
- Hover or keyboard-focus an orb to show the matching profile’s compact details beside that orb. Moving into the detail panel must keep it open.
- When a managed session requests permission or an answer, open its options beside the originating profile’s orb. Preserve account/project context and route replies using the existing request ID. Pending agents remain visible even when not pinned.
- Retain the full workspace/settings as a secondary entry and the optional macOS island mode.

## Implementation decisions

The default `floating` mode becomes the edge dock, so existing users receive the new layout without resetting their database. Right edge is the default; left edge is selectable. Circle size, gap, padding, panel width, edge inset and hover-close delay live in settings/default configuration.

Use one transparent native window: narrow while collapsed, wider only while a detail/approval panel is open. Resizing preserves the screen-edge anchor and vertical position, so hovering does not move the orb away from the pointer. Details are constrained to the current display. The dock scrolls if many agents exceed the display height.

Pending interactions are mapped to profiles through their originating managed session, never by provider name alone. A hidden profile with a pending interaction is temporarily added to the dock. One anchored panel is active at a time; all other pending orbs retain badges. Hover exit is delayed to allow crossing the gap into the panel; active confirmations remain open until resolved or explicitly dismissed.

## Verification

Test quota-ring math including 0/100/unknown, profile-safe interaction mapping, hidden pending agents, ring markup and scoped approval cards. Test native edge geometry for expansion, both sides and smaller displays. Run the full frontend/Rust suites, production build and macOS packaging; attempt visual inspection through available UI automation.

## Completed validation

Frontend tests (13) and the full Rust suite passed, including vertical-position preservation and restoring unsent question drafts. Production TypeScript/Vite build and macOS app packaging passed. Native visual inspection remains unavailable in this automation environment; no screenshot-based verification is claimed.
