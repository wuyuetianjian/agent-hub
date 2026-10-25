import test from "node:test";
import assert from "node:assert/strict";
import { remaining, resetLabel } from "../src/lib/format.ts";
test("unknown quotas remain unknown, zero usage remains a real full allowance", () => {
  assert.equal(remaining(undefined), null);
  assert.equal(remaining(0), 100);
  assert.equal(remaining(25), 75);
  assert.equal(remaining(120), 0);
  assert.equal(remaining(-1), null);
  assert.equal(remaining(NaN), null);
});
test("elapsed reset signals refresh rather than inventing replenishment", () => {
  assert.equal(resetLabel(10, 20, "en"), "Awaiting refresh");
  assert.equal(resetLabel(null, 20, "en"), "Reset unavailable");
  assert.equal(resetLabel(3620, 20, "en"), "1h 0m");
});
