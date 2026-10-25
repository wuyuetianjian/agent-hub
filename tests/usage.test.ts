import test from "node:test";
import assert from "node:assert/strict";
import { dateInput, usageIntervals } from "../src/lib/usage.ts";

test("today uses local hourly boundaries and seven days use daily buckets", () => {
  const now = new Date(2026, 9, 25, 13, 20);
  const hourly = usageIntervals("today", "", "", now);
  assert.equal(hourly.length, 24);
  assert.equal(new Date(hourly[0].start * 1000).getHours(), 0);
  const daily = usageIntervals("7d", "", "", now);
  assert.equal(daily.length, 7);
  assert.equal(dateInput(new Date(daily[0].start * 1000)), "2026-10-19");
  assert.ok(
    daily.every(
      (bucket, index) => index === 0 || daily[index - 1].end === bucket.start,
    ),
  );
});
test("custom range includes the entire final local day and rejects invalid dates", () => {
  const now = new Date(2026, 9, 25, 13);
  const buckets = usageIntervals("custom", "2026-10-20", "2026-10-22", now);
  assert.equal(buckets.length, 3);
  assert.equal(dateInput(new Date(buckets.at(-1)!.end * 1000)), "2026-10-23");
  assert.throws(() =>
    usageIntervals("custom", "2026-02-30", "2026-10-22", now),
  );
  assert.throws(() =>
    usageIntervals("custom", "2026-10-26", "2026-10-27", now),
  );
  assert.throws(() =>
    usageIntervals("custom", "2025-01-01", "2026-10-22", now),
  );
});
test("DST fallback day retains both repeated hours without overlap", () => {
  const previous = process.env.TZ;
  process.env.TZ = "America/New_York";
  try {
    const now = new Date(2026, 10, 1, 12);
    const buckets = usageIntervals("today", "", "", now);
    assert.equal(buckets.length, 25);
    assert.ok(
      buckets.every(
        (bucket, index) =>
          index === 0 || buckets[index - 1].end === bucket.start,
      ),
    );
  } finally {
    if (previous === undefined) delete process.env.TZ;
    else process.env.TZ = previous;
  }
});
