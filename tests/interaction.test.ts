import test from "node:test";
import assert from "node:assert/strict";
import {
  defaultSessionMode,
  approvalConnection,
} from "../src/lib/interaction.ts";
import type { SessionInfo } from "../src/types.ts";
test("Codex starts in the approval-capable mode by default while respecting explicit terminal preference", () => {
  assert.equal(defaultSessionMode({ id: "codex" }), "structured");
  assert.equal(defaultSessionMode({ id: "claude" }), "terminal");
  assert.equal(
    defaultSessionMode({ id: "codex", defaultSessionMode: "terminal" }),
    "terminal",
  );
});
test("approval connectivity distinguishes managed channels from terminal and external history", () => {
  const base = {
    id: "s",
    profileId: "work",
    status: "running",
    readOnly: false,
  } as SessionInfo;
  assert.equal(
    approvalConnection("work", [{ ...base, interactionChannel: "codex-rpc" }]),
    "connected",
  );
  assert.equal(
    approvalConnection("other", [{ ...base, interactionChannel: "codex-rpc" }]),
    "none",
  );
  assert.equal(
    approvalConnection("work", [{ ...base, interactionChannel: "terminal" }]),
    "terminal",
  );
  assert.equal(
    approvalConnection("work", [
      { ...base, readOnly: true, interactionChannel: "history" },
    ]),
    "history",
  );
});
