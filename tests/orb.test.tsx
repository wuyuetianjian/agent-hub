import React from "react";
import test from "node:test";
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import { QuotaOrb } from "../src/components/QuotaOrb";
import { SettingsPanel } from "../src/components/SettingsPanel";
import { AgentDock } from "../src/components/AgentDock";
import { translator } from "../src/i18n";
import defaults from "../config/defaults.json";
import type { AgentView, Dashboard, Settings } from "../src/types";
const provider = defaults.providers.find((p) => p.id === "codex")!;
const settings = defaults.settings as Settings;
function agent(percent: number | null): AgentView {
  return {
    provider,
    profile: {
      id: "work",
      providerId: "codex",
      label: "Work account",
      executable: "",
      args: [],
      configDir: "",
      quotaCommand: "",
      quotaArgs: [],
      quotaFile: "",
      enabled: true,
    },
    discovery: { installed: true, executable: "codex", configFound: true },
    snapshot: {
      profileId: "work",
      account: "Work account",
      plan: null,
      status: "ready",
      source: "test",
      message: "",
      fetchedAt: 1,
      windows: [
        {
          label: "5h",
          remainingPercent: percent,
          used: null,
          limit: null,
          unit: "percent",
          resetsAt: null,
          windowMinutes: 300,
        },
      ],
    },
  };
}
test("orb places provider icon inside an accessible remaining-quota ring", () => {
  const html = renderToStaticMarkup(
    <QuotaOrb
      agent={agent(75)}
      settings={settings}
      pending={1}
      active={false}
      onHover={() => {}}
      onLeave={() => {}}
      onSelect={() => {}}
    />,
  );
  assert.ok(html.includes('role="progressbar"'));
  assert.ok(html.includes('aria-valuenow="75"'));
  assert.ok(html.includes("/icons/codex.svg"));
  assert.ok(html.includes("75%"));
  assert.ok(html.includes("orb-request-count"));
});
test("unknown quota uses an unknown ring without claiming zero remaining", () => {
  const html = renderToStaticMarkup(
    <QuotaOrb
      agent={agent(null)}
      settings={settings}
      active={false}
      onHover={() => {}}
      onLeave={() => {}}
      onSelect={() => {}}
    />,
  );
  assert.ok(html.includes("orb-ring unknown"));
  assert.ok(!html.includes('aria-valuenow="0"'));
});
test("pending confirmation appears beside the originating orb immediately", () => {
  const data: Dashboard = {
    agents: [agent(75)],
    settings,
    sessions: [
      {
        id: "session-1",
        profileId: "work",
        providerId: "codex",
        cwd: "/work",
        mode: "structured",
        status: "running",
        readOnly: false,
        title: "Work",
        source: "managed",
        updatedAt: 1,
        startedAt: 1,
      },
    ],
    interactions: [
      {
        id: "request-1",
        sessionId: "session-1",
        providerId: "codex",
        profileLabel: "Work account",
        cwd: "/work",
        title: "Execute command",
        detail: "touch file",
        kind: "permission",
        choices: [
          { id: "allow", label: "Allow once" },
          { id: "deny", label: "Deny" },
        ],
        questions: [],
        createdAt: 1,
        expiresAt: Date.now() / 1000 + 60,
      },
    ],
    platform: "macos",
  };
  const html = renderToStaticMarkup(
    <AgentDock
      data={data}
      t={translator("en")}
      busy={[]}
      onPanelChange={() => {}}
      onRefresh={() => {}}
      onDetails={() => {}}
      onSettings={() => {}}
      onSessions={() => {}}
      onRespond={async () => {}}
    />,
  );
  assert.ok(html.includes("dock-popover approval-popover"));
  assert.ok(html.includes("Work account"));
  assert.ok(html.includes("/work"));
  assert.ok(html.includes("Allow once"));
});
import { ApprovalCard } from "../src/components/ApprovalCard";
test("question text is restored from its request draft after switching views", () => {
  const request = {
    id: "draft-request",
    sessionId: "s",
    providerId: "codex",
    profileLabel: "Work",
    cwd: "/work",
    title: "Question",
    detail: "",
    kind: "question",
    choices: [],
    questions: [
      { id: "q", title: "Project name?", options: [], multiple: false },
    ],
    createdAt: 1,
    expiresAt: Date.now() / 1000 + 60,
  };
  const html = renderToStaticMarkup(
    <ApprovalCard
      request={request}
      t={translator("en")}
      draft={{
        answers: { q: ["Partly written answer"] },
        freeText: { q: "Partly written answer" },
      }}
      onRespond={async () => {}}
    />,
  );
  assert.ok(html.includes('value="Partly written answer"'));
});

test("dock settings expose separate resting and hover size controls", () => {
  const html = renderToStaticMarkup(
    <SettingsPanel
      data={{
        settings,
        platform: "macos",
        agents: [agent(75)],
        sessions: [],
        interactions: [],
      }}
      t={translator("en")}
      onSave={async () => {}}
    />,
  );
  assert.match(html, /aria-label="Resting size"/);
  assert.match(html, /aria-label="Hover size"/);
  assert.match(html, /Changes apply immediately/);
});
