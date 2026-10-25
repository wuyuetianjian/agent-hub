import {
  X,
  ArrowUpRight,
  RefreshCw,
  Clock3,
  TerminalSquare,
  Pin,
  PinOff,
} from "lucide-react";
import type {
  AgentView,
  Interaction,
  Reply,
  Settings,
  SessionInfo,
} from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
import { ApprovalCard, type ApprovalDraft } from "./ApprovalCard";
import { statusText } from "./QuotaCard";
import { summaryWindow } from "../lib/summary";
import { resetLabel } from "../lib/format";
export function DockDetails({
  current,
  s,
  requests,
  sessions,
  drafts = {},
  onDraftChange,
  t,
  busy,
  now,
  pinned,
  onPin,
  onClose,
  onDetails,
  onRefresh,
  onSessions,
  onRespond,
}: {
  current: AgentView;
  s: Settings;
  requests: Interaction[];
  sessions: SessionInfo[];
  drafts?: Record<string, ApprovalDraft>;
  onDraftChange?: (id: string, draft: ApprovalDraft) => void;
  t: T;
  busy: string[];
  now: number;
  pinned?: boolean;
  onPin?: () => void;
  onClose: () => void;
  onDetails: (id: string) => void;
  onRefresh: (id: string) => void;
  onSessions: (id: string) => void;
  onRespond: (id: string, reply: Reply) => Promise<void>;
}) {
  const primary = summaryWindow(current.snapshot.windows, s.summaryPolicy);
  const windows = [
    ...(primary ? [primary] : []),
    ...current.snapshot.windows.filter((window) => window !== primary),
  ];
  return (
    <>
      <header className="dock-popover-header">
        <AgentIcon provider={current.provider} size={30} />
        <div>
          <strong>{current.provider.name}</strong>
          <span>{current.snapshot.account || current.profile.label}</span>
          {current.snapshot.plan && <small>{current.snapshot.plan}</small>}
        </div>
        <button
          className="icon-button"
          title={t("details")}
          onClick={() => onDetails(current.profile.id)}
        >
          <ArrowUpRight size={15} />
        </button>
        {onPin && (
          <button
            className={`icon-button ${pinned ? "on" : ""}`}
            title={t(pinned ? "unpinDetails" : "pinDetails")}
            onClick={onPin}
          >
            {pinned ? <PinOff size={14} /> : <Pin size={14} />}
          </button>
        )}
        <button className="icon-button" title={t("close")} onClick={onClose}>
          <X size={14} />
        </button>
      </header>
      {requests.length ? (
        <div className="dock-approval-list">
          {requests.map((request) => (
            <ApprovalCard
              key={request.id}
              request={request}
              draft={drafts[request.id]}
              onDraftChange={(draft) => onDraftChange?.(request.id, draft)}
              provider={current.provider}
              t={t}
              expandedDetails
              onRespond={(reply) => onRespond(request.id, reply)}
            />
          ))}
        </div>
      ) : (
        <div className="dock-quota-details">
          <span
            className={`dock-status ${current.snapshot.status === "ready" ? "ready" : ""}`}
          >
            {statusText(current.snapshot.status, t)}
          </span>
          {current.snapshot.windows.length ? (
            windows.map((quota, i) => (
              <div className="dock-quota-window" key={`${quota.label}-${i}`}>
                <div>
                  <span>{quota.label}</span>
                  <strong>
                    {quota.remainingPercent == null
                      ? "—"
                      : `${Math.round(quota.remainingPercent)}%`}{" "}
                    <small>{t("remaining")}</small>
                  </strong>
                </div>
                <div className="quota-track">
                  <span
                    style={{
                      width: `${quota.remainingPercent ?? 0}%`,
                      background: current.provider.color,
                    }}
                  />
                </div>
                <p>
                  <Clock3 size={11} />
                  {resetLabel(quota.resetsAt, now, s.locale)}
                </p>
              </div>
            ))
          ) : (
            <p className="dock-empty-quota">
              {current.snapshot.message || t("emptyHint")}
            </p>
          )}
          {current.snapshot.fetchedAt > 0 && (
            <small className="dock-updated">
              {t("updated")} ·{" "}
              {new Date(current.snapshot.fetchedAt * 1000).toLocaleTimeString(
                s.locale,
              )}
            </small>
          )}
          <footer>
            <span>{current.snapshot.source || current.provider.publisher}</span>
            <button
              className="icon-button"
              title={t("refresh")}
              disabled={busy.includes(current.profile.id)}
              onClick={() => onRefresh(current.profile.id)}
            >
              <RefreshCw
                size={13}
                className={busy.includes(current.profile.id) ? "spin" : ""}
              />
            </button>
          </footer>
          <button
            className="dock-sessions-link"
            onClick={() => onSessions(current.profile.id)}
          >
            <TerminalSquare size={13} />
            {t("sessions")}
            <span>
              {
                sessions.filter(
                  (session) => session.profileId === current.profile.id,
                ).length
              }
            </span>
            <ArrowUpRight size={12} />
          </button>
        </div>
      )}
    </>
  );
}
