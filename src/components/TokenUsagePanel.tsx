import { useEffect, useState, useRef } from "react";
import { RefreshCw, BarChart3 } from "lucide-react";
import { call } from "../api";
import type { Dashboard, UsageReport } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
import {
  dateInput,
  usageIntervals,
  formatTokens,
  type UsageRange,
} from "../lib/usage";
export function TokenUsagePanel({ data, t }: { data: Dashboard; t: T }) {
  const [range, setRange] = useState<UsageRange>(
    data.settings.usageHistoryDays >= 7 ? "7d" : "today",
  );
  const [from, setFrom] = useState(dateInput(new Date()));
  const [to, setTo] = useState(dateInput(new Date()));
  const contextKey = JSON.stringify([
    range,
    from,
    to,
    dateInput(new Date()),
    data.settings.usageHistoryDays,
    data.settings.locale,
    data.agents.map((agent) => [
      agent.profile.id,
      agent.profile.configDir,
      agent.profile.enabled,
    ]),
  ]);
  const [storedReport, setStoredReport] = useState<{
    key: string;
    data: UsageReport;
  } | null>(null);
  const report = storedReport?.key === contextKey ? storedReport.data : null;
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<number | null>(null);
  const [refreshKey, refresh] = useState(0);
  const request = useRef(0);
  useEffect(() => {
    let active = true;
    const load = async () => {
      const id = ++request.current;
      let intervals;
      try {
        intervals = usageIntervals(
          range,
          from,
          to,
          new Date(),
          data.settings.usageHistoryDays,
        );
      } catch {
        setError(t("usageInvalidRange"));
        setStoredReport(null);
        setLoading(false);
        return;
      }
      setLoading(true);
      setError("");
      try {
        const result = await call<UsageReport>("get_usage", { intervals });
        if (active && request.current === id) {
          setStoredReport({ key: contextKey, data: result });
          setSelected(null);
        }
      } catch (error) {
        if (active && request.current === id) {
          setError(String(error));
          setStoredReport(null);
        }
      } finally {
        if (active && request.current === id) setLoading(false);
      }
    };
    void load();
    const timer = setInterval(
      () => void load(),
      data.settings.usageRefreshSeconds * 1000,
    );
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [contextKey, refreshKey, data.settings.usageRefreshSeconds]);
  const available = report?.sources.some((source) => source.hasData) ?? false;
  const count = (value: number | undefined) =>
    available && value != null
      ? formatTokens(value, data.settings.locale)
      : "—";
  const max = Math.max(
    1,
    ...(report?.buckets.map((bucket) => bucket.tokens.total) ?? []),
  );
  const chosen = selected == null ? null : report?.buckets[selected];
  const hourly = Boolean(
    report?.buckets.length &&
    report.buckets[0].end - report.buckets[0].start <= 3600,
  );
  const axisLabel = (at: number) =>
    hourly
      ? new Date(at * 1000).toLocaleTimeString(data.settings.locale, {
          hour: "2-digit",
          minute: "2-digit",
        })
      : new Date(at * 1000).toLocaleDateString(data.settings.locale);
  const label = (start: number, end: number) =>
    `${new Date(start * 1000).toLocaleString(data.settings.locale, { timeZoneName: "short" })} – ${new Date(end * 1000).toLocaleString(data.settings.locale, { timeZoneName: "short" })}`;
  return (
    <section className="token-usage-panel" aria-label={t("tokenUsage")}>
      <header className="token-usage-heading">
        <div>
          <span className="eyebrow">TOKENS</span>
          <h3>{t("tokenUsage")}</h3>
        </div>
        <div className="usage-range-controls">
          <div
            className="usage-range-tabs"
            role="group"
            aria-label={t("usageRange")}
          >
            {(["today", "7d", "30d", "custom"] as UsageRange[]).map((value) => (
              <button
                key={value}
                className={range === value ? "active" : ""}
                aria-pressed={range === value}
                onClick={() => setRange(value)}
              >
                {t(
                  value === "today"
                    ? "usageToday"
                    : value === "7d"
                      ? "usage7d"
                      : value === "30d"
                        ? "usage30d"
                        : "usageCustom",
                )}
              </button>
            ))}
          </div>
          <button
            className="icon-button"
            title={t("refresh")}
            disabled={loading}
            onClick={() => refresh((value) => value + 1)}
          >
            <RefreshCw size={14} className={loading ? "spin" : ""} />
          </button>
        </div>
      </header>
      {range === "custom" && (
        <div className="usage-date-range">
          <label>
            {t("usageFrom")}
            <input
              type="date"
              value={from}
              max={dateInput(new Date())}
              onChange={(event) => setFrom(event.target.value)}
            />
          </label>
          <label>
            {t("usageTo")}
            <input
              type="date"
              value={to}
              max={dateInput(new Date())}
              onChange={(event) => setTo(event.target.value)}
            />
          </label>
        </div>
      )}
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <div className="token-metrics">
        {(
          [
            ["total", "usageTotal"],
            ["input", "usageInput"],
            ["output", "usageOutput"],
            ["cacheRead", "usageCache"],
          ] as const
        ).map(([key, title]) => (
          <div key={key}>
            <span>{t(title)}</span>
            <strong
              title={
                available && report
                  ? report.total[key].toLocaleString(data.settings.locale)
                  : undefined
              }
            >
              {count(report?.total[key])}
            </strong>
            <small>tokens</small>
          </div>
        ))}
      </div>
      {loading && !report ? (
        <div className="usage-empty">
          <BarChart3 size={22} />
          {t("usageLoading")}
        </div>
      ) : available && report ? (
        <>
          <div className="usage-trend-heading">
            <strong>{t("usageTrend")}</strong>
            <small>
              {t(hourly ? "usageHourly" : "usageDaily")} · {t("usageLocalTime")}
            </small>
          </div>
          <div
            className="usage-chart"
            role="group"
            aria-label={`${t("usageTrend")} · tokens`}
          >
            {report.buckets.map((bucket, index) => (
              <button
                key={bucket.start}
                className={selected === index ? "selected" : ""}
                title={`${label(bucket.start, bucket.end)}: ${bucket.tokens.total.toLocaleString(data.settings.locale)} tokens`}
                aria-label={`${label(bucket.start, bucket.end)}: ${bucket.tokens.total} tokens`}
                onMouseEnter={() => setSelected(index)}
                onFocus={() => setSelected(index)}
                onClick={() => setSelected(index)}
              >
                <span
                  style={{ height: `${(bucket.tokens.total / max) * 100}%` }}
                />
              </button>
            ))}
          </div>
          <div className="usage-chart-axis">
            <span>{axisLabel(report.buckets[0].start)}</span>
            <span>
              {formatTokens(max, data.settings.locale)} tokens ·{" "}
              {t("usageScale")}
            </span>
            <span>{axisLabel(report.buckets.at(-1)!.end - 1)}</span>
          </div>
          <div className="usage-chart-detail" aria-live="polite">
            {chosen ? (
              <>
                <span>{label(chosen.start, chosen.end)}</span>
                <strong>
                  {chosen.tokens.total.toLocaleString(data.settings.locale)}{" "}
                  tokens
                </strong>
              </>
            ) : (
              <span>{t("usageInspect")}</span>
            )}
          </div>
          {report.agents.length > 0 && (
            <div className="usage-agent-list">
              <div className="usage-agent-header">
                <span>{t("allAgents")}</span>
                <span>{t("usageInput")}</span>
                <span>{t("usageOutput")}</span>
                <span>{t("usageTotal")}</span>
              </div>
              {report.agents.map((row) => {
                const agent = data.agents.find(
                  (agent) => agent.profile.id === row.profileId,
                );
                return (
                  <div key={row.profileId} className="usage-agent-row">
                    <span>
                      {agent && (
                        <AgentIcon provider={agent.provider} size={23} />
                      )}
                      <span>
                        {agent?.provider.name ?? row.providerId}
                        <small>{agent?.profile.label ?? row.profileId}</small>
                      </span>
                    </span>
                    <span>{count(row.tokens.input)}</span>
                    <span>{count(row.tokens.output)}</span>
                    <strong>{count(row.tokens.total)}</strong>
                  </div>
                );
              })}
            </div>
          )}
        </>
      ) : (
        <div className="usage-empty">
          <BarChart3 size={22} />
          <strong>{t("usageNoData")}</strong>
          <span>{t("usageSourcesHint")}</span>
        </div>
      )}
      {report?.partial && (
        <p className="usage-notice" role="status">
          {t("usagePartial")}
        </p>
      )}
      <footer className="usage-footnote">
        <span>{t("usageAccounting")}</span>
        <details>
          <summary>{t("usageSources")}</summary>
          {report?.sources.map((source) => (
            <div key={source.profileId}>
              <strong>
                {data.agents.find(
                  (agent) => agent.profile.id === source.profileId,
                )?.profile.label ?? source.providerId}
              </strong>
              <span>
                {t(
                  source.status === "retained"
                    ? "usageRetained"
                    : source.status === "ready"
                      ? "usageReady"
                      : source.status === "partial"
                        ? "usagePartialSource"
                        : source.status === "sharedSource"
                          ? "usageShared"
                          : source.status === "unsupported"
                            ? "usageUnsupported"
                            : "usageMissing",
                )}
                {source.files > 0
                  ? ` · ${source.files} ${t("usageFiles")}`
                  : ""}
              </span>
            </div>
          ))}
        </details>
      </footer>
    </section>
  );
}
