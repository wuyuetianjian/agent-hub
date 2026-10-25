# Hover details and account token timeline

User reports that details still require a click and that the Codex/OpenAI floating details lack token usage over time.

The existing full detail component is already used for hover, but opening relies entirely on webview mouse-enter delivery; clicking also silently pins the card, blocking later hover changes. Keep ordinary click equivalent to hover, reserve pinning for the explicit pin control, and add native cursor sampling for inactive floating windows. Convert physical desktop coordinates into each window's logical coordinates using its own scale; never activate the workspace. Use a database-backed, validated `dockPointerPollMs` default from mountable config. Keep native mouse events as a fallback when a compositor cannot supply global cursor coordinates. Mouse travel into the detail window cancels the close delay; leaving both closes an unpinned card; approvals remain open.

Reuse the existing TokenUsagePanel in floating full details for the selected account. Add an optional profileId to get_usage, validate the account, and restrict collection/report scopes to that profile. Preserve main-page aggregate behavior. Floating views offer Today, 7d, 30d and custom ranges, hourly/daily bins, actual input/output/cache totals, loading/no-data states, with compact responsive styling. Switch accounts without stale results or totals from other agents. Do not invent token counts from quota percentages.

Validate physical/logical geometry, negative monitor coordinates, cursor leaving both windows, profile-scoped usage, hover control state, frontend/native builds and a desktop UI check where available. Prior uncommitted changes must be preserved; the environment previously denied writing Git metadata.

Native UI verification: the actual Codex floating panel loaded today's recorded token totals and 24 hourly buckets, with the account-scoped backend (no mocks or model requests). The initial layout placed the chart below totals; adjust compact visual order so the timeline appears before the totals and remains visible at the configured 580px panel height. Testing the no-click path is limited by the desktop driver's synthetic pointer behavior; do not count clicking/fixing the pin control as a hover pass.

## Final verification

72 Rust tests and 28 frontend tests passed; TypeScript/Vite production build, macOS native bundle, and ZIP integrity passed. Actual native account details displayed a 24-bucket hourly Today chart and a seven-bucket daily 7d chart. Final screenshot verified the hour chart is visible in the initial 580px panel viewport. Explicit pinning was used only to hold the pane for data inspection and was removed after testing. Native cursor geometry and account isolation have automated coverage; synthetic desktop pointer interactions did not independently establish a no-click hover visual pass. Windows/Linux native UI testing was not performed on this Mac.

Git metadata remains read-only in this environment (previous `index.lock` creation was denied); preserve all source changes and both requirement documents in the distribution patch instead of claiming a commit.
