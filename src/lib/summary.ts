import type { QuotaWindow } from "../types.ts";
export function summaryWindow(
  windows: QuotaWindow[],
  policy: "shortest" | "lowest",
): QuotaWindow | undefined {
  if (policy === "lowest")
    return (
      windows
        .filter(
          (w) =>
            w.remainingPercent != null && Number.isFinite(w.remainingPercent),
        )
        .sort((a, b) => a.remainingPercent! - b.remainingPercent!)[0] ??
      windows[0]
    );
  return [...windows].sort(
    (a, b) =>
      (a.windowMinutes && a.windowMinutes > 0 ? a.windowMinutes : Infinity) -
      (b.windowMinutes && b.windowMinutes > 0 ? b.windowMinutes : Infinity),
  )[0];
}
