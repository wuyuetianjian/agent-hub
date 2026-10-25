import {
  ExternalLink,
  RefreshCw,
  Settings2,
  Play,
  Clock3,
  ArrowLeft,
} from "lucide-react";
import type { AgentView, Settings } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
import { resetLabel } from "../lib/format";
import { statusText } from "./QuotaCard";
export function AgentDetail({
  agent,
  settings,
  t,
  busy,
  onBack,
  onRefresh,
  onConfigure,
  onLaunch,
  onUsage,
}: {
  agent: AgentView;
  settings: Settings;
  t: T;
  busy: boolean;
  onBack: () => void;
  onRefresh: () => void;
  onConfigure: () => void;
  onLaunch: () => void;
  onUsage: () => void;
}) {
  return (
    <section className="agent-detail">
      <button className="back-button" onClick={onBack}>
        <ArrowLeft size={14} />
        {t("back")}
      </button>
      <div className="detail-title">
        <AgentIcon provider={agent.provider} size={48} />
        <div>
          <h2>{agent.provider.name}</h2>
          <p>{agent.snapshot.account || agent.profile.label}</p>
        </div>
        <button
          className="icon-button"
          onClick={onConfigure}
          title={t("configure")}
        >
          <Settings2 size={17} />
        </button>
      </div>
      <div className="detail-chips">
        <span
          className={`status-chip ${agent.snapshot.status === "ready" ? "good" : ""}`}
        >
          <i />
          {statusText(agent.snapshot.status, t)}
        </span>
        {agent.snapshot.plan && (
          <span className="status-chip">{agent.snapshot.plan}</span>
        )}
      </div>
      <div className="section-label">{t("allWindows")}</div>
      {agent.snapshot.windows.length ? (
        agent.snapshot.windows.map((w, i) => (
          <div className="window-quota" key={`${w.label}-${i}`}>
            <div>
              <span>{w.label}</span>
              <strong>
                {w.remainingPercent == null
                  ? "—"
                  : `${Math.round(w.remainingPercent)}%`}{" "}
                <small>{t("remaining")}</small>
              </strong>
            </div>
            <div className="quota-track">
              <span
                style={{
                  width: `${w.remainingPercent ?? 0}%`,
                  background: agent.provider.color,
                }}
              />
            </div>
            <p>
              <Clock3 size={12} />
              {resetLabel(w.resetsAt, Date.now() / 1000, settings.locale)}
              {w.used != null && (
                <span>
                  {w.used}
                  {w.limit != null ? ` / ${w.limit}` : ""} {w.unit}
                </span>
              )}
            </p>
          </div>
        ))
      ) : (
        <div className="empty-quota">
          <span className="empty-orbit">◌</span>
          <h3>{t("emptyQuota")}</h3>
          <p>{agent.snapshot.message || t("emptyHint")}</p>
        </div>
      )}
      <dl className="metadata">
        <div>
          <dt>{t("source")}</dt>
          <dd>{agent.snapshot.source || "—"}</dd>
        </div>
        <div>
          <dt>{t("updated")}</dt>
          <dd>
            {agent.snapshot.fetchedAt
              ? new Date(agent.snapshot.fetchedAt * 1000).toLocaleTimeString(
                  settings.locale === "zh" ? "zh-CN" : "en-US",
                )
              : "—"}
          </dd>
        </div>
      </dl>
      <div className="detail-actions">
        <button className="primary-button" onClick={onRefresh} disabled={busy}>
          <RefreshCw size={14} className={busy ? "spin" : ""} />
          {t("refresh")}
        </button>
        <button className="secondary-button" onClick={onUsage}>
          <ExternalLink size={14} />
          {t("openUsage")}
        </button>
      </div>
      <button className="full-button" onClick={onLaunch}>
        <Play size={13} />
        {t("newSession")}
      </button>
    </section>
  );
}
