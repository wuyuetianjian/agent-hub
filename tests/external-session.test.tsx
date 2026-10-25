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
