import test from "node:test";
import assert from "node:assert/strict";
import {
  ringOffset,
  requestsForProfile,
  dockAgents,
  profileActivity,
} from "../src/lib/dock.ts";
import type { AgentView, Interaction, SessionInfo } from "../src/types.ts";
test("ring preserves full, exhausted and unknown quota", () => {
  assert.equal(ringOffset(100, 28), 0);
  assert.equal(ringOffset(0, 28), 2 * Math.PI * 28);
  assert.equal(ringOffset(null, 28), null);
  assert.equal(ringOffset(-1, 28), null);
  assert.equal(ringOffset(50, 28), Math.PI * 28);
});
const sessions = [
  { id: "work", profileId: "work-profile" },
  { id: "personal", profileId: "personal-profile" },
] as SessionInfo[];
const requests = [
  { id: "approval-work", sessionId: "work", providerId: "codex" },
  { id: "approval-personal", sessionId: "personal", providerId: "codex" },
] as Interaction[];
test("approval options are scoped to the account profile rather than shared provider name", () => {
  assert.deepEqual(
    requestsForProfile("work-profile", sessions, requests).map((r) => r.id),
    ["approval-work"],
  );
  assert.deepEqual(requestsForProfile("missing", sessions, requests), []);
});
test("unpinning an agent cannot hide its pending confirmation", () => {
  const agents = [
    { profile: { id: "work-profile" }, provider: { id: "codex" } },
    { profile: { id: "other" }, provider: { id: "claude" } },
  ] as AgentView[];
  assert.deepEqual(
    dockAgents(agents, ["claude"], sessions, requests).map((a) => a.profile.id),
    ["other", "work-profile"],
  );
});

test("imported session history does not imply live agent activity", () => {
  assert.equal(
    profileActivity(
      "work",
      [
        {
          id: "s",
          profileId: "work",
          readOnly: true,
          status: "running",
        } as SessionInfo,
      ],
      [],
    ),
    "idle",
  );
  assert.equal(
    profileActivity(
      "work",
      [
        {
          id: "s",
          profileId: "work",
          readOnly: false,
          status: "running",
        } as SessionInfo,
      ],
      [],
    ),
    "running",
  );
  assert.equal(
    profileActivity(
      "work",
      [
        {
          id: "s",
          profileId: "work",
          readOnly: false,
          status: "running",
        } as SessionInfo,
      ],
      [{ sessionId: "s" } as Interaction],
    ),
    "waiting",
  );
});
