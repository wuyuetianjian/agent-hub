import { ChevronRight, CircleAlert } from "lucide-react";
import type { AgentView, Settings } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
import { summaryWindow } from "../lib/summary";
import { resetLabel } from "../lib/format";
export function statusText(status: string, t: T) {
  return t(
    ([
      "ready",
      "stale",
      "error",
      "notInstalled",
      "notLoggedIn",
      "unsupported",
      "unavailable",
      "loading",
    ].includes(status)
      ? status
      : "unavailable") as Parameters<T>[0],
  );
}
export function QuotaCard({
  agent,
  settings,
  t,
  selected,
  onClick,
}: {
  agent: AgentView;
  settings: Settings;
  t: T;
  selected: boolean;
  onClick: () => void;
}) {
  const w = summaryWindow(agent.snapshot.windows, settings.summaryPolicy);
  const percent = w?.remainingPercent;
  const low = percent != null && percent <= settings.warningPercent;
  const stale = agent.snapshot.status === "stale";
  return (
    <button
      className={`quota-card ${selected ? "selected" : ""} ${low ? "low" : ""} ${stale ? "stale" : ""}`}
      onClick={onClick}
    >
      <span className="quota-line">
        <AgentIcon provider={agent.provider} />
        <span className="agent-label">
          <strong>{agent.provider.name}</strong>
          <span>
            {agent.snapshot.account ||
              (agent.profile.label !== agent.provider.name &&
                agent.profile.label) ||
              agent.provider.publisher}
          </span>
        </span>
        <span className="quota-value">
          {percent != null ? (
            <>
              <b>
                {Math.round(percent)}
                <small>%</small>
              </b>
              <span>
                {w?.label} · {t("remaining")}
              </span>
            </>
          ) : (
            <>
              <b className="unknown-number">—</b>
              <span>{statusText(agent.snapshot.status, t)}</span>
            </>
          )}
        </span>
        <ChevronRight size={14} className="row-chevron" />
      </span>
      <span className={`quota-track ${percent == null ? "unknown-track" : ""}`}>
        <span
          style={{
            width: `${percent ?? 0}%`,
            background: low ? "var(--warning)" : agent.provider.color,
          }}
        />
      </span>
      <span className="quota-foot">
        {w ? (
          <>
            <span>
              {stale ? (
                <>
                  <CircleAlert size={11} />
                  {t("stale")}
                </>
              ) : (
                resetLabel(w.resetsAt, Date.now() / 1000, settings.locale)
              )}
            </span>
            <span className="source-dot" />
          </>
        ) : (
          <span>
            {agent.discovery.installed ? t("connected") : t("emptyQuota")}
          </span>
        )}
      </span>
    </button>
  );
}
