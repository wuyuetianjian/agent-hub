import React from "react";
import test from "node:test";
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import { ApprovalCard } from "../src/components/ApprovalCard";
import { translator } from "../src/i18n";
test("approval identifies the original account and project before the decision buttons", () => {
  const html = renderToStaticMarkup(
    <ApprovalCard
      request={{
        id: "r",
        sessionId: "session-12345678",
        providerId: "claude",
        profileLabel: "Work account A",
        cwd: "/projects/project-A",
        title: "Bash",
        detail: "touch example",
        kind: "permission",
        choices: [
          { id: "allow", label: "Allow once" },
          { id: "deny", label: "Deny" },
        ],
        questions: [],
        createdAt: 1,
        expiresAt: Date.now() / 1000 + 30,
      }}
      t={translator("en")}
      onRespond={async () => {}}
    />,
  );
  assert.ok(html.includes("Work account A"));
  assert.ok(html.includes("/projects/project-A"));
  assert.ok(html.indexOf("Work account A") < html.indexOf("Allow once"));
});

test("approval rule and cancellation labels are readable and localized", () => {
  const html = renderToStaticMarkup(
    <ApprovalCard
      request={{
        id: "r",
        sessionId: "session",
        providerId: "codex",
        profileLabel: "Test",
        cwd: "/test",
        title: "Approval",
        detail: "command",
        kind: "permission",
        choices: [
          { id: "1", label: "Allow command rule" },
          { id: "2", label: "Cancel" },
          { id: "3", label: "Allow for session" },
        ],
        questions: [],
        createdAt: 1,
        expiresAt: Date.now() / 1000 + 30,
      }}
      t={translator("zh")}
      onRespond={async () => {}}
    />,
  );
  assert.ok(html.includes("允许并保存命令规则"));
  assert.ok(html.includes("取消"));
  assert.ok(html.includes("本会话允许"));
  assert.ok(!html.includes("acceptWithExecpolicyAmendment"));
});

test("approval body renders only reason and command while escaping markup", () => {
  const html = renderToStaticMarkup(
    <ApprovalCard
      request={{
        id: "r",
        sessionId: "session",
        providerId: "codex",
        profileLabel: "Test",
        cwd: "/test",
        title: "Approval",
        detail: JSON.stringify({
          reason: "Review <script>alert(1)</script>",
          command: "echo '<unsafe>'",
          threadId: "hidden-protocol-id",
          availableDecisions: ["accept"],
        }),
        kind: "permission",
        choices: [{ id: "0", label: "Allow once" }],
        questions: [],
        createdAt: 1,
        expiresAt: Date.now() / 1000 + 30,
      }}
      t={translator("en")}
      expandedDetails
      onRespond={async () => {}}
    />,
  );
  assert.ok(html.includes("Review &lt;script&gt;"));
  assert.ok(html.includes("&lt;unsafe&gt;"));
  assert.ok(!html.includes("hidden-protocol-id"));
  assert.ok(!html.includes("availableDecisions"));
  assert.ok(!html.includes("<details"));
  assert.ok(!html.includes("<script>"));
});
