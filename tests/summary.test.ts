import test from "node:test";
import assert from "node:assert/strict";
import { summaryWindow } from "../src/lib/summary.ts";
const weekly = {
  label: "7d",
  remainingPercent: 10,
  used: null,
  limit: null,
  unit: "percent",
  resetsAt: null,
  windowMinutes: 10080,
};
const hourly = {
  ...weekly,
  label: "5h",
  remainingPercent: 80,
  windowMinutes: 300,
};
test("shortest period selects five-hour quota regardless of source ordering", () => {
  assert.equal(summaryWindow([weekly, hourly], "shortest")?.label, "5h");
});
test("lowest policy ignores unknown quota and preserves real zero remaining", () => {
  assert.equal(
    summaryWindow(
      [
        { ...weekly, remainingPercent: null },
        hourly,
        { ...weekly, remainingPercent: 0 },
      ],
      "lowest",
    )?.remainingPercent,
    0,
  );
});
