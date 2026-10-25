import type { UsageInterval } from "../types";
export type UsageRange = "today" | "7d" | "30d" | "custom";
export function dateInput(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}
export function usageIntervals(
  range: UsageRange,
  from: string,
  to: string,
  now: Date = new Date(),
  maxDays = 90,
): UsageInterval[] {
  const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  let start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (range === "7d") start.setDate(start.getDate() - 6);
  if (range === "30d") start.setDate(start.getDate() - 29);
  if (range === "custom") {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(from) || !/^\d{4}-\d{2}-\d{2}$/.test(to))
      throw new Error("invalidRange");
    start = new Date(`${from}T00:00:00`);
    const last = new Date(`${to}T00:00:00`);
    if (
      !Number.isFinite(+start) ||
      !Number.isFinite(+last) ||
      dateInput(start) !== from ||
      dateInput(last) !== to
    )
      throw new Error("invalidRange");
    end.setTime(+last);
    end.setDate(end.getDate() + 1);
  }
  const earliest = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  earliest.setDate(earliest.getDate() - maxDays + 1);
  const latest = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  if (+start >= +end || +start < +earliest || +end > +latest)
    throw new Error("invalidRange");
  const singleDay = dateInput(start) === dateInput(new Date(+end - 1));
  const buckets: UsageInterval[] = [];
  let cursor = +start;
  while (cursor < +end) {
    const next = new Date(cursor);
    if (singleDay) next.setTime(cursor + 3600000);
    else next.setDate(next.getDate() + 1);
    const endAt = Math.min(+next, +end);
    buckets.push({
      start: Math.floor(cursor / 1000),
      end: Math.floor(endAt / 1000),
    });
    cursor = endAt;
  }
  return buckets;
}
export function formatTokens(value: number, locale: string): string {
  return new Intl.NumberFormat(locale, {
    notation: value >= 10000 ? "compact" : "standard",
    maximumFractionDigits: 1,
  }).format(value);
}
