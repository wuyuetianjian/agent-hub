import { useEffect, useRef, useState } from "react";
import {
  Settings2,
  GripVertical,
  X,
  ArrowUpRight,
  RefreshCw,
  Clock3,
  TerminalSquare,
} from "lucide-react";
import type { Dashboard, Reply } from "../types";
import type { T } from "../i18n";
import { dockAgents, requestsForProfile } from "../lib/dock";
import { resetLabel } from "../lib/format";
import { QuotaOrb } from "./QuotaOrb";
import { AgentIcon } from "./AgentIcon";
import { ApprovalCard, type ApprovalDraft } from "./ApprovalCard";
import { statusText } from "./QuotaCard";
export function AgentDock({
  data,
  drafts = {},
  onDraftChange,
  t,
  busy,
  onPanelChange,
  onRefresh,
  onDetails,
  onSettings,
  onSessions,
  onRespond,
}: {
  data: Dashboard;
  drafts?: Record<string, ApprovalDraft>;
  onDraftChange?: (id: string, draft: ApprovalDraft) => void;
  t: T;
  busy: string[];
  onPanelChange: (open: boolean) => void;
  onRefresh: (id: string) => void;
  onDetails: (id: string) => void;
  onSettings: () => void;
  onSessions: (id: string) => void;
  onRespond: (id: string, reply: Reply) => Promise<void>;
}) {
  const s = data.settings;
  const agents = dockAgents(
    data.agents,
    s.visibleProviders,
    data.sessions,
    data.interactions,
  );
  const [active, setActive] = useState<string | null>(() => {
    const request = data.interactions.find(
      (r) => r.expiresAt > Date.now() / 1000,
    );
    return request
      ? (data.sessions.find(
          (session) => session.id === request.sessionId && !session.readOnly,
        )?.profileId ?? null)
      : null;
  });
  const [dismissed, setDismissed] = useState<string[]>([]);
  const [now, setNow] = useState(Date.now() / 1000);
  const [panelHeight, setPanelHeight] = useState(300);
  const [anchor, setAnchor] = useState(100);
  const [viewportHeight, setViewportHeight] = useState(
    typeof window !== "undefined" ? window.innerHeight : s.dockPopupHeight,
  );
  const panelRef = useRef<HTMLDivElement>(null);
  const railRef = useRef<HTMLDivElement>(null);
  const pointerInside = useRef(false);
  const previousRequestCount = useRef(0);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const current = agents.find((a) => a.profile.id === active);
  const live = data.interactions.filter((r) => r.expiresAt > now);
  const requests = current
    ? requestsForProfile(current.profile.id, data.sessions, live)
    : [];
  const signature = live.map((r) => r.id).join("|");
  const cancelClose = () => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
    closeTimer.current = undefined;
  };
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => clearInterval(timer);
  }, []);
  useEffect(() => {
    const resize = () => setViewportHeight(window.innerHeight);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    onPanelChange(Boolean(current));
  }, [Boolean(current), onPanelChange]);
  useEffect(
    () => () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
      onPanelChange(false);
    },
    [onPanelChange],
  );
  useEffect(() => {
    if (requests.some((r) => !dismissed.includes(r.id))) return;
    const first = live.find((r) => !dismissed.includes(r.id));
    if (!first) return;
    const session = data.sessions.find(
      (session) => session.id === first.sessionId && !session.readOnly,
    );
    if (session && agents.some((a) => a.profile.id === session.profileId))
      setActive(session.profileId);
  }, [signature, active, dismissed.join("|")]);
  useEffect(() => {
    if (!panelRef.current) return;
    const observer = new ResizeObserver((entries) => {
      setPanelHeight(entries[0].target.getBoundingClientRect().height);
    });
    observer.observe(panelRef.current);
    return () => observer.disconnect();
  }, [active, requests.map((r) => r.id).join("|")]);
  useEffect(() => {
    if (
      previousRequestCount.current > 0 &&
      requests.length === 0 &&
      !pointerInside.current &&
      !panelRef.current?.contains(document.activeElement)
    )
      setActive(null);
    previousRequestCount.current = requests.length;
  }, [requests.length]);
  function updateAnchor(id: string) {
    const node = railRef.current?.querySelector<HTMLElement>(
      `[data-profile="${CSS.escape(id)}"]`,
    );
    if (node) {
      const box = node.getBoundingClientRect();
      setAnchor(box.top + box.height / 2);
    }
  }
  useEffect(() => {
    if (active) updateAnchor(active);
  }, [active, viewportHeight, agents.length]);
  function show(id: string, explicit = false) {
    pointerInside.current = true;
    cancelClose();
    if (!explicit && requests.length) return;
    setActive(id);
    updateAnchor(id);
  }
  function leave() {
    pointerInside.current = false;
    cancelClose();
    if (requests.length) return;
    closeTimer.current = setTimeout(() => {
      if (panelRef.current?.contains(document.activeElement)) return;
      setActive(null);
    }, s.dockHoverDelayMs);
  }
  function dismiss() {
    cancelClose();
    setDismissed((ids) => [...new Set([...ids, ...requests.map((r) => r.id)])]);
    setActive(null);
  }
  const top = Math.max(
    s.dockPadding,
    Math.min(
      anchor - panelHeight / 2,
      viewportHeight - panelHeight - s.dockPadding,
    ),
  );
  const style = {
    "--dock-size": `${s.dockSize}px`,
    "--dock-gap": `${s.dockGap}px`,
    "--dock-padding": `${s.dockPadding}px`,
    "--dock-panel-width": `${s.dockPanelWidth}px`,
    "--dock-rail-width": `${s.dockSize + 2 * s.dockPadding}px`,
    "--dock-chrome-height": `${s.dockChromeHeight}px`,
  } as React.CSSProperties;
  return (
    <div
      className={`agent-dock side-${s.dockSide} ${current ? "panel-open" : ""}`}
      style={style}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          dismiss();
        }
      }}
    >
      <div className="dock-rail" ref={railRef}>
        <div className="dock-drag-handle" data-tauri-drag-region>
          <GripVertical size={13} />
        </div>
        <div
          className="dock-orbs"
          onScroll={() => {
            if (active) updateAnchor(active);
          }}
        >
          {agents.map((agent) => (
            <div key={agent.profile.id} data-profile={agent.profile.id}>
              <QuotaOrb
                agent={agent}
                settings={s}
                pending={
                  requestsForProfile(agent.profile.id, data.sessions, live)
                    .length
                }
                active={agent.profile.id === active}
                onHover={() => show(agent.profile.id)}
                onLeave={leave}
                onSelect={() => show(agent.profile.id, true)}
              />
            </div>
          ))}
        </div>
        <div className="dock-controls">
          <button
            className="dock-settings"
            title={t("settings")}
            onClick={onSettings}
          >
            <Settings2 size={17} />
          </button>
        </div>
      </div>
      {current && (
        <div
          ref={panelRef}
          className={`dock-popover ${requests.length ? "approval-popover" : ""}`}
          role="dialog"
          aria-label={current.provider.name}
          style={
            {
              top,
              maxHeight: Math.max(200, viewportHeight - 2 * s.dockPadding),
              "--dock-arrow-top": `${Math.max(20, Math.min(anchor - top, panelHeight - 20))}px`,
            } as React.CSSProperties
          }
          onMouseEnter={() => {
            pointerInside.current = true;
            cancelClose();
          }}
          onMouseLeave={leave}
          onFocusCapture={cancelClose}
        >
          <header className="dock-popover-header">
            <AgentIcon provider={current.provider} size={30} />
            <div>
              <strong>{current.provider.name}</strong>
              <span>{current.snapshot.account || current.profile.label}</span>
            </div>
            <button
              className="icon-button"
              title={t("details")}
              onClick={() => onDetails(current.profile.id)}
            >
              <ArrowUpRight size={15} />
            </button>
            <button
              className="icon-button"
              title={t("close")}
              onClick={dismiss}
            >
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
                current.snapshot.windows.map((quota, i) => (
                  <div
                    className="dock-quota-window"
                    key={`${quota.label}-${i}`}
                  >
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
              <footer>
                <span>
                  {current.snapshot.source || current.provider.publisher}
                </span>
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
                    data.sessions.filter(
                      (session) => session.profileId === current.profile.id,
                    ).length
                  }
                </span>
                <ArrowUpRight size={12} />
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
