import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import config from "../src-tauri/tauri.conf.json";
import capability from "../src-tauri/capabilities/main.json";

test("completion window is independent, non-activating and authorized for IPC", () => {
  const window = config.app.windows.find(
    (item) => item.label === "task-completion",
  );
  assert.ok(window);
  assert.equal(window.url, "index.html?completion=1");
  assert.equal(window.visible, false);
  assert.equal(window.focus, false);
  assert.equal(window.skipTaskbar, true);
  assert.ok(capability.windows.includes("task-completion"));
});

test("native completion reconciliation dispatches to the UI thread without a cross-thread window mutex", () => {
  // Architectural guard: Tauri getters synchronously wait for the UI thread.
  // A worker-held gate also acquired by a UI command deadlocks before getters run.
  const source = readFileSync(
    new URL("../src-tauri/src/desktop.rs", import.meta.url),
    "utf8",
  );
  const wrapper = source.slice(
    source.indexOf("fn sync_completion_popup("),
    source.indexOf("fn sync_completion_popup_on_main("),
  );
  assert.match(wrapper, /run_on_main_thread/);
  assert.doesNotMatch(source, /completion_gate/);
});
