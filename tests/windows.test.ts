import test from "node:test";
import assert from "node:assert/strict";
import config from "../src-tauri/tauri.conf.json";

test("only the workspace participates in the OS taskbar", () => {
  const windows = config.app.windows;
  assert.equal(
    windows.find((window) => window.label === "main")?.skipTaskbar,
    true,
  );
  assert.equal(
    windows.find((window) => window.label === "dock-panel")?.skipTaskbar,
    true,
  );
  assert.equal(
    windows.find((window) => window.label === "workspace")?.skipTaskbar,
    false,
  );
  assert.equal(
    windows.find((window) => window.label === "main")?.focusable,
    false,
  );
});

test("floating details and workspace use independent webviews", () => {
  assert.equal(
    config.app.windows.find((window) => window.label === "dock-panel")?.url,
    "index.html?panel=1",
  );
  assert.equal(
    config.app.windows.find((window) => window.label === "workspace")?.url,
    "index.html?workspace=1",
  );
});
