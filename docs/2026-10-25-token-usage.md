# Token usage on the main overview

Add real historical token use alongside remaining subscription quotas. Defaults: Today, Last 7 days, Last 30 days and custom local-date range; hourly buckets for a single day and daily buckets for longer periods. Show total, input, output and cached input, a time-series bar chart and per-agent/account breakdown. Cached and reasoning counts are subsets where the provider reports them; never add them twice to total tokens. No price estimates or fabricated data.

Read configured local usage log paths for Codex JSONL, Claude Code JSONL and Gemini CLI chat JSON. Resolve account roots using explicit profile config directory, provider config environment, then configured defaults. Persist only usage metadata and ingestion cursors in SQLite: timestamps, provider/profile/session identifiers and token counts, never prompts or replies. Incremental JSONL ingestion keeps partial trailing lines for the next scan. Deduplicate copied/archived logs and repeated cumulative counters; Claude streamed snapshots use per-message maxima. Imported Codex cumulative counts become per-event deltas; a reset starts a new counter segment. Gemini reasoning is added to candidate output exactly once.

Bound file discovery, bytes per scan and history/query ranges through database-backed settings/default configuration. Run collection outside the UI thread and serialize overlapping scans. Preserve observed history if logs disappear. Bind history queries to currently configured source roots so profile changes do not combine accounts. Report source availability and truncation/scan errors; missing sources are not zero use. Use a separate command queried only by the main overview, so the floating windows do not trigger history scans.

The hover readability correction remains part of this change: independent floating detail window inherits theme variables, has an opaque background, and shows full details on hover; clicks only pin it.

Validation: cumulative delta/dedup/reset math, Claude cache/message snapshots, Gemini thought counts, partial lines, archive copies, repeated collection/restart, time bounds and timezone buckets, missing sources. Frontend chart/accessibility and range tests; full test/build/package checks.

Profile attribution follows configured local directories; logs do not reliably identify historical account switches inside one shared directory. UI labels use the configured profile, not an inferred email. Anthropic message IDs remain identical when conversations are forked, so those source IDs are deduplicated independently of the containing session ID.

Retention keeps a one-day boundary buffer so complete local calendar days remain available across timezone and daylight-saving transitions; the UI still limits the selected number of calendar days to the configured history range.

For a resumed Codex log beginning with a pre-existing cumulative counter, the first observed event uses `last_token_usage` when available, while retaining the cumulative baseline for subsequent deltas. This avoids attributing prior-session usage to the first resumed call.

## Verification results

61 Rust tests and 25 frontend tests passed, including native-feature test compilation, duplicate/resumed/forked usage records, partial lines and byte budgets, retention expansion, profile-root isolation and daylight-saving buckets. Production TypeScript/Vite build and native macOS app build passed. A read-only probe of the machine's real Codex logs found usage records; two consecutive scans returned identical counts. The probe used a temporary database and was removed.

Review corrections invalidate ingestion cursors when history retention changes, charge malformed JSON reads against the shared budget and bind displayed reports to their date/profile query identity. Detached hover surfaces now inherit theme variables and display full content directly without requiring a pin click. Native visual verification remains unavailable in the current automation environment.
