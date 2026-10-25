import React from "react";
import test from "node:test";
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import { ExternalSession } from "../src/components/ExternalSession";
import { translator } from "../src/i18n";
test("existing session shows origin metadata without terminal or approval inputs", () => {
  const html = renderToStaticMarkup(
    <ExternalSession
      session={{
        id: "external:codex:thread-1",
        profileId: "codex",
        providerId: "codex",
        cwd: "/project/existing",
        mode: "external",
        status: "external",
        startedAt: 1,
        updatedAt: 2,
        readOnly: true,
        title: "Existing desktop task",
        source: "desktop",
      }}
      t={translator("en")}
      locale="en"
    />,
  );
  assert.ok(html.includes("Existing desktop task"));
  assert.ok(html.includes("/project/existing"));
  assert.ok(html.includes("desktop"));
  assert.ok(html.includes("read-only"));
  assert.ok(!html.includes("<input"));
  assert.ok(!html.includes("Allow once"));
});

test("existing Codex history offers explicit approval connection without a terminal takeover", () => {
  const session = {
    id: "external:codex:thread",
    profileId: "codex",
    providerId: "codex",
    cwd: "/project",
    mode: "external",
    status: "external",
    startedAt: 1,
    updatedAt: 2,
    readOnly: true,
    title: "Original",
    source: "desktop",
  };
  const html = renderToStaticMarkup(
    <ExternalSession
      session={session}
      t={translator("en")}
      locale="en"
      onConnect={async () => ({ installCommand: "test-only" })}
    />,
  );
  assert.ok(html.includes("Connect all Codex windows"));
  assert.ok(!html.includes("Terminal input"));
  const pending = renderToStaticMarkup(
    <ExternalSession
      session={{
        ...session,
        mode: "hook",
        status: "hook-pending",
        readOnly: false,
      }}
      t={translator("en")}
      locale="en"
    />,
  );
  assert.ok(pending.includes("connection is not verified yet"));
  const recover = renderToStaticMarkup(
    <ExternalSession
      session={{
        ...session,
        mode: "hook",
        status: "hook-pending",
        readOnly: false,
      }}
      t={translator("en")}
      locale="en"
      onConnect={async () => ({ installCommand: "test-only" })}
    />,
  );
  assert.ok(recover.includes("Show connection instructions"));
});

test("user-level coverage is visible from an unrelated imported window without claiming its callback arrived", () => {
  const html = renderToStaticMarkup(
    <ExternalSession
      session={{
        id: "external:codex:another-thread",
        profileId: "codex",
        providerId: "codex",
        cwd: "/another/project",
        mode: "external",
        status: "external",
        readOnly: true,
        title: "Another window",
        source: "desktop",
        startedAt: 1,
        updatedAt: 2,
      }}
      t={translator("en")}
      locale="en"
      coverage={{ profileId: "codex", installed: true }}
      onDisableAll={async () => {}}
    />,
  );
  assert.ok(html.includes("All-window hook installed"));
  assert.ok(html.includes("Pause all-window approvals"));
  assert.ok(!html.includes("An approval hook arrived"));
});
