import test from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import defaults from "../config/defaults.json";
import { translator } from "../src/i18n";
import type { Dashboard, Settings } from "../src/types";

test("overview offers time ranges and does not invent token totals before collection", async () => {
  const previous = globalThis.window;
  Object.defineProperty(globalThis, "window", {
    value: {},
    configurable: true,
    writable: true,
  });
  try {
    const { TokenUsagePanel } =
      await import("../src/components/TokenUsagePanel");
    const data: Dashboard = {
      settings: defaults.settings as Settings,
      platform: "macos",
      agents: [],
      sessions: [],
      interactions: [],
    };
    const html = renderToStaticMarkup(
      <TokenUsagePanel data={data} t={translator("en")} />,
    );
    assert.match(html, /Token usage/);
    assert.match(html, /Last 7 days/);
    assert.match(html, /Last 30 days/);
    assert.match(html, /Recorded total/);
    assert.match(html, /—/);
    assert.ok(!html.includes("<strong>0</strong>"));
    const floating = renderToStaticMarkup(
      <TokenUsagePanel
        data={data}
        profileId="codex-work"
        compact
        t={translator("en")}
      />,
    );
    assert.ok(floating.includes("compact-token-usage"));
    assert.match(floating, /class="active" aria-pressed="true">Today/);
    assert.ok(!floating.includes("usage-agent-list"));
  } finally {
    if (previous === undefined) Reflect.deleteProperty(globalThis, "window");
    else
      Object.defineProperty(globalThis, "window", {
        value: previous,
        configurable: true,
        writable: true,
      });
  }
});
