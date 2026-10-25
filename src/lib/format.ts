export function remaining(used: number | null | undefined): number | null {
  return used == null || !Number.isFinite(used) || used < 0
    ? null
    : Math.max(0, 100 - used);
}
export function resetLabel(
  reset: number | null,
  now: number,
  locale: string,
): string {
  const zh = locale === "zh";
  if (reset == null) return zh ? "重置时间未知" : "Reset unavailable";
  if (reset <= now) return zh ? "等待额度刷新" : "Awaiting refresh";
  const minutes = Math.ceil((reset - now) / 60);
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  return days > 0 ? `${days}d ${hours}h` : `${hours}h ${minutes % 60}m`;
}
