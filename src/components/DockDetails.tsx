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
import { TokenUsagePanel } from "./TokenUsagePanel";
import { ApprovalCard, type ApprovalDraft } from "./ApprovalCard";
import { statusText } from "./QuotaCard";
import { approvalConnection } from "../lib/interaction";
import { profileActivity } from "../lib/dock";
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
  compact = false,
  onSession,
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
  compact?: boolean;
  onSession?: (id: string) => void;
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
  const activity = profileActivity(current.profile.id, sessions, requests);
  const activityLabel = t(
    activity === "waiting"
      ? "hudWaiting"
      : activity === "running"
        ? "hudRunning"
        : "hudIdle",
  );
  if (compact && !requests.length)
    return (
      <section
        className="hud-peek"
        aria-label={`${current.provider.name} ${t("quota")}`}
      >
        <header>
          <AgentIcon provider={current.provider} size={20} />
          <strong>{current.provider.name}</strong>
          <span className={`hud-session-state ${activity}`}>
            {activityLabel}
          </span>
          <button className="icon-button" title={t("close")} onClick={onClose}>
            <X size={12} />
          </button>
        </header>
        <button
          className="hud-peek-open"
          onClick={onPin ?? (() => onDetails(current.profile.id))}
          aria-label={t("hudExpand")}
        >
          <div className="hud-peek-quotas">
            {windows.length ? (
              windows.map((quota, index) => (
                <span key={`${quota.label}-${index}`}>
                  <small>{quota.label}</small>
                  <strong>
                    {quota.remainingPercent == null
                      ? "—"
                      : `${Math.round(quota.remainingPercent)}%`}
                  </strong>
                  <small>{t("remaining")}</small>
                </span>
              ))
            ) : (
              <span>{statusText(current.snapshot.status, t)}</span>
            )}
          </div>
          <footer>
            <span>{current.snapshot.account || current.profile.label}</span>
            <ArrowUpRight size={12} />
          </footer>
        </button>
        {primary && (
          <small className="hud-peek-reset">
            <Clock3 size={10} />
            {resetLabel(primary.resetsAt, now, s.locale)}
          </small>
        )}
      </section>
    );
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
      <div className={`hud-card-state ${activity}`}>
        <span className="hud-state-dot" />
        {activityLabel}
      </div>
      <p className="hud-approval-connection">
        {t(
          approvalConnection(current.profile.id, sessions) === "connected"
            ? "approvalConnected"
            : approvalConnection(current.profile.id, sessions) === "history"
              ? "approvalHistoryOnly"
              : approvalConnection(current.profile.id, sessions) === "terminal"
                ? "approvalInTerminal"
                : "approvalNoSession",
        )}
      </p>
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
          <TokenUsagePanel
            key={current.profile.id}
            data={{ settings: s, agents: [current] }}
            profileId={current.profile.id}
            compact
            t={t}
          />
          {onSession &&
            sessions.some(
              (session) => session.profileId === current.profile.id,
            ) && (
              <div className="hud-session-list">
                {sessions
                  .filter((session) => session.profileId === current.profile.id)
                  .sort(
                    (a, b) =>
                      Number(
                        requests.some((request) => request.sessionId === b.id),
                      ) -
                        Number(
                          requests.some(
                            (request) => request.sessionId === a.id,
                          ),
                        ) || b.updatedAt - a.updatedAt,
                  )
                  .map((session) => (
                    <button
                      key={session.id}
                      onClick={() => onSession(session.id)}
                      title={session.cwd}
                    >
                      <TerminalSquare size={12} />
                      <span>
                        {session.title || session.cwd || t("untitledSession")}
                        <small>
                          {session.readOnly
                            ? t("externalSession")
                            : session.status === "running"
                              ? t("hudRunning")
                              : t("hudIdle")}
                        </small>
                      </span>
                      <ArrowUpRight size={12} />
                    </button>
                  ))}
              </div>
            )}
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
