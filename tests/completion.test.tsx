import React from "react";
import test from "node:test";
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import { CompletionCard } from "../src/components/CompletionWindow";
import { CompletionInbox } from "../src/lib/completion";
import { translator } from "../src/i18n";
import type { TaskCompletion } from "../src/types";
const first: TaskCompletion = {
  id: "a",
  sessionId: "session-a",
  turnId: "turn-a",
  profileId: "codex",
  profileLabel: "Work account",
  cwd: "/projects/alpha",
  completedAt: 1,
};
const second = { ...first, id: "b", sessionId: "session-b", turnId: "turn-b" };
function setup(initial = [first, second]) {
  let rows = initial;
  const opened: string[] = [];
  const inbox = new CompletionInbox(
    {
      list: async () => rows,
      dismiss: async (id) => {
        rows = rows.filter((r) => r.id !== id);
      },
      open: async (sessionId) => {
        opened.push(sessionId);
      },
    },
    () => {},
  );
  return { inbox, opened };
}
test("completion identifies account, project and session and provides localized accessible actions", () => {
  const html = renderToStaticMarkup(
    <CompletionCard
      completion={first}
      count={2}
      busy={false}
      error=""
      t={translator("en")}
      onOpen={() => {}}
      onDismiss={() => {}}
    />,
  );
  for (const text of [
    "Task completed",
    "Work account",
    "/projects/alpha",
    "session-a",
    "View session",
    "Dismiss notification",
    "2",
  ])
    assert.ok(html.includes(text), text);
  assert.ok(html.includes('role="status"'));
  const zh = renderToStaticMarkup(
    <CompletionCard
      completion={first}
      count={1}
      busy={true}
      error="Oops"
      t={translator("zh")}
      onOpen={() => {}}
      onDismiss={() => {}}
    />,
  );
  assert.ok(zh.includes("任务已完成"));
  assert.ok(zh.includes("查看会话"));
  assert.ok(zh.includes("disabled"));
  assert.ok(zh.includes("Oops"));
});
test("reload recovers FIFO queue and opening dismisses only that completion", async () => {
  const { inbox, opened } = setup();
  await inbox.refresh();
  assert.deepEqual(inbox.state.items, [first, second]);
  await inbox.act(first, "open");
  assert.deepEqual(opened, ["session-a"]);
  assert.deepEqual(inbox.state.items, [second]);
  await inbox.act(second, "dismiss");
  assert.deepEqual(opened, ["session-a"]);
  assert.deepEqual(inbox.state.items, []);
});
test("failed navigation retains completion and permits retry", async () => {
  let fail = true;
  let dismissed = false;
  const inbox = new CompletionInbox(
    {
      list: async () => (dismissed ? [] : [first]),
      open: async () => {
        if (fail) throw Error("Workspace unavailable");
      },
      dismiss: async () => {
        dismissed = true;
      },
    },
    () => {},
  );
  await inbox.refresh();
  await inbox.act(first, "open");
  assert.equal(dismissed, false);
  assert.deepEqual(inbox.state.items, [first]);
  assert.match(inbox.state.error, /Workspace unavailable/);
  assert.equal(inbox.state.busy, false);
  fail = false;
  await inbox.act(first, "open");
  assert.deepEqual(inbox.state.items, []);
});
test("double clicks do not open or dismiss twice", async () => {
  let release!: () => void;
  let opens = 0;
  let dismissals = 0;
  const inbox = new CompletionInbox(
    {
      list: async () => [first],
      open: async () => {
        opens++;
        await new Promise<void>((resolve) => {
          release = resolve;
        });
      },
      dismiss: async () => {
        dismissals++;
      },
    },
    () => {},
  );
  const pending = inbox.act(first, "open");
  await inbox.act(first, "open");
  assert.equal(opens, 1);
  release();
  await pending;
  assert.equal(dismissals, 1);
});
test("older refresh cannot resurrect dismissed or replace newer queue state", async () => {
  let resolveOld!: (items: TaskCompletion[]) => void;
  let calls = 0;
  const inbox = new CompletionInbox(
    {
      list: () =>
        ++calls === 1
          ? new Promise((resolve) => {
              resolveOld = resolve;
            })
          : Promise.resolve([second]),
      open: async () => {},
      dismiss: async () => {},
    },
    () => {},
  );
  const old = inbox.refresh();
  await inbox.refresh();
  resolveOld([first]);
  await old;
  assert.deepEqual(inbox.state.items, [second]);
});

test("failed acknowledgement keeps notification available and clears busy state", async () => {
  const inbox = new CompletionInbox(
    {
      list: async () => [first],
      open: async () => {},
      dismiss: async () => {
        throw Error("Acknowledgement unavailable");
      },
    },
    () => {},
  );
  await inbox.refresh();
  await inbox.act(first, "dismiss");
  assert.deepEqual(inbox.state.items, [first]);
  assert.equal(inbox.state.busy, false);
  assert.match(inbox.state.error, /Acknowledgement unavailable/);
});

test("completion arriving during acknowledgement remains queued", async () => {
  let rows = [first];
  const inbox = new CompletionInbox(
    {
      list: async () => rows,
      open: async () => {},
      dismiss: async () => {
        rows = [second];
      },
    },
    () => {},
  );
  await inbox.refresh();
  await inbox.act(first, "dismiss");
  assert.deepEqual(inbox.state.items, [second]);
});

test("successful recovery clears a transient queue query error", async () => {
  let unavailable = true;
  const inbox = new CompletionInbox(
    {
      list: async () => {
        if (unavailable) throw Error("Queue unavailable");
        return [first];
      },
      open: async () => {},
      dismiss: async () => {},
    },
    () => {},
  );
  await inbox.refresh();
  assert.match(inbox.state.error, /Queue unavailable/);
  unavailable = false;
  await inbox.refresh();
  assert.deepEqual(inbox.state.items, [first]);
  assert.equal(inbox.state.error, "");
});
