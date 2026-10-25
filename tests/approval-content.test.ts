import test from "node:test";
import assert from "node:assert/strict";
import { approvalContent } from "../src/lib/approval-content";
test("shows upstream reason and readable commands without protocol metadata", () => {
  assert.deepEqual(
    approvalContent({
      detail: JSON.stringify({
        reason: "请确认测试命令",
        command: "/bin/zsh -lc test",
        commandActions: [{ type: "unknown", command: "node -e 'print()'" }],
        availableDecisions: ["accept"],
        threadId: "private-protocol-id",
      }),
    }),
    { reason: "请确认测试命令", actions: ["node -e 'print()'"] },
  );
});
test("handles missing, malformed and legacy action payloads without dumping JSON", () => {
  for (const detail of ["not-json", "null", "[]", "{}"])
    assert.deepEqual(approvalContent({ detail }), {
      reason: null,
      actions: [],
    });
  assert.deepEqual(
    approvalContent({
      detail: JSON.stringify({ reason: null, command: "echo test" }),
    }),
    { reason: null, actions: ["echo test"] },
  );
  assert.deepEqual(
    approvalContent({ detail: JSON.stringify({ file_path: "/test/a.txt" }) }),
    { reason: null, actions: ["/test/a.txt"] },
  );
  assert.deepEqual(
    approvalContent({
      detail: JSON.stringify({
        changes: { "/test/b.txt": { type: "update" } },
      }),
    }),
    { reason: null, actions: ["/test/b.txt"] },
  );
});
