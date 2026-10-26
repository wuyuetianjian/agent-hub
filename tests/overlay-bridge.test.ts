import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const bridgePath = new URL("../src/lib/overlay-bridge.ts", import.meta.url);
async function bridge() {
  assert.ok(existsSync(bridgePath), "overlay bridge implementation exists");
  return import("../src/lib/overlay-bridge.ts");
}

test("host proxy validates requests and prevents recursive internal commands", async () => {
  const { installOverlayHost } = await bridge();
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  const host: any = {};
  await installOverlayHost(
    host,
    async (command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return undefined as any;
    },
  );
  assert.deepEqual(calls, [{ command: "overlay_host_ready", args: undefined }]);
  for (const request of [
    null,
    {},
    { id: 0, command: "get_dashboard", args: {} },
    { id: Infinity, command: "get_dashboard", args: {} },
    { id: 1.5, command: "get_dashboard", args: {} },
    { id: 1, command: "plugin:foo", args: {} },
    { id: 1, command: "get_dashboard", args: [] },
    { id: 1, command: "get_dashboard", args: new Date() },
    ...[
      "overlay_reply",
      "overlay_host",
      "overlay_host_ready",
      "overlay_helper_start",
    ].map((command) => ({ id: 1, command, args: {} })),
  ]) {
    await assert.rejects(() => host.__ORBIT_HOST_INVOKE__(request));
  }
  assert.equal(calls.length, 1);
  await host.__ORBIT_HOST_INVOKE__({
    id: 2,
    command: "get_dashboard",
    args: {},
  });
  assert.deepEqual(calls.slice(1), [
    { command: "get_dashboard", args: {} },
    { command: "overlay_reply", args: { id: 2, ok: true, value: null } },
  ]);
});

test("host proxy preserves values, serializes command errors, and allows helper URL/dialog commands", async () => {
  const { installOverlayHost } = await bridge();
  const replies: unknown[] = [];
  const host: any = {};
  await installOverlayHost(
    host,
    async (command: string, args?: Record<string, unknown>) => {
      if (command === "overlay_reply") replies.push(args);
      if (command === "get_dashboard") throw new Error("offline");
      return command === "overlay_choose_directory"
        ? "/tmp/project"
        : (undefined as any);
    },
  );
  await host.__ORBIT_HOST_INVOKE__({
    id: 1,
    command: "get_dashboard",
    args: {},
  });
  await host.__ORBIT_HOST_INVOKE__({
    id: 2,
    command: "overlay_choose_directory",
    args: {},
  });
  await host.__ORBIT_HOST_INVOKE__({
    id: 3,
    command: "overlay_open_url",
    args: { url: "https://example.com" },
  });
  assert.deepEqual(replies, [
    { id: 1, ok: false, value: "offline" },
    { id: 2, ok: true, value: "/tmp/project" },
    { id: 3, ok: true, value: null },
  ]);
});

test("helper API is desktop-capable without Tauri and routes invokes and events through injected bridge", async () => {
  const calls: unknown[] = [];
  const off = () => {};
  (globalThis as any).window = {
    __ORBIT_OVERLAY__: {
      invoke: async (command: string, args: unknown) => {
        calls.push([command, args]);
        return "value";
      },
      listen: async (event: string, callback: (event: unknown) => void) => {
        callback({ event, payload: 42 });
        return off;
      },
    },
  };
  const api = await import("../src/api.ts");
  assert.equal(api.isDesktop, true);
  assert.equal(await api.call("get_dashboard"), "value");
  let payload: unknown;
  assert.equal(
    await api.listen("orbit:test", (event) => {
      payload = event.payload;
    }),
    off,
  );
  assert.equal(payload, 42);
  await api.openUrl("https://example.com");
  await api.chooseDirectory();
  assert.deepEqual(calls, [
    ["get_dashboard", {}],
    ["overlay_open_url", { url: "https://example.com" }],
    ["overlay_choose_directory", {}],
  ]);
  await assert.rejects(() => api.openUrl("file:///tmp/private"));
  delete (globalThis as any).window;
});

test("entry point queries host role before mounting and App uses the portable listener", () => {
  const main = readFileSync(
    new URL("../src/main.tsx", import.meta.url),
    "utf8",
  );
  assert.match(main, /await invoke<boolean>\("overlay_host"\)/);
  assert.match(main, /await installOverlayHost\(window, invoke\)/);
  assert.match(main, /catch/);
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(app, /from "@tauri-apps\/api\/event"/);
});

test("workspace reports native overlay failures with an initial status query and cleaned-up live subscription", () => {
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  const effect = app.slice(
    app.indexOf("const reportOverlayError"),
    app.indexOf("const reportOverlayError") + 1500,
  );
  assert.match(app, /if \(!isDesktop \|\| !workspaceWindow\) return;/);
  assert.match(effect, /Native floating windows unavailable:/);
  assert.match(effect, /listen<string>\("orbit:overlay-error"/);
  assert.match(effect, /call<string \| null>\("overlay_status"\)/);
  assert.match(effect, /if \(!disposed && diagnostic\)/);
  assert.match(effect, /if \(disposed\) fn\(\)/);
  assert.match(effect, /disposed = true;\s+off\?\.\(\)/);
});
