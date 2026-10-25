import { useEffect, useRef, useState } from "react";
import { Settings2, GripVertical, LockKeyhole } from "lucide-react";
import type { Dashboard, Reply } from "../types";
import type { T } from "../i18n";
import { dockAgents, requestsForProfile } from "../lib/dock";
import { QuotaOrb } from "./QuotaOrb";
import { type ApprovalDraft } from "./ApprovalCard";
import { DockDetails } from "./DockDetails";
export function AgentDock({
  data,
  drafts = {},
  onDraftChange,
  t,
  busy,
  onPanelChange,
  detached = false,
  onPanelTarget,
  panelPresence,
  onDrag,
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
  detached?: boolean;
  onPanelTarget?: (target: {
    profileId: string | null;
    anchor: number;
    pinned: boolean;
  }) => void;
  panelPresence?: {
    inside: boolean;
    dismiss?: boolean;
    togglePin?: boolean;
    sequence: number;
  };
  onDrag?: (
    phase: "begin" | "move" | "end",
    delta?: { dx: number; dy: number },
  ) => void;
  onRefresh: (id: string) => void;
  onDetails: (id: string) => void;
  onSettings: () => void;
  onSessions: (id: string) => void;
  onRespond: (id: string, reply: Reply) => Promise<void>;
}) {
  const s = data.settings;
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const popupInside = useRef(false);
  const size = s.dockHoverSize;
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
  const dragOrigin = useRef<{ x: number; y: number } | null>(null);
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
    onPanelTarget?.({ profileId: current?.profile.id ?? null, anchor, pinned });
  }, [current?.profile.id, anchor, pinned, onPanelTarget]);
  useEffect(() => {
    if (!panelPresence) return;
    popupInside.current = panelPresence.inside;
    if (panelPresence.dismiss) dismiss();
    else if (panelPresence.togglePin) setPinned((v) => !v);
    else if (panelPresence.inside) cancelClose();
    else leave();
  }, [panelPresence?.sequence]);
  useEffect(() => {
    onPanelChange(Boolean(current));
  }, [Boolean(current), onPanelChange]);
  useEffect(
    () => () => {
      if (closeTimer.current) clearTimeout(closeTimer.current);
      onPanelChange(false);
      onPanelTarget?.({ profileId: null, anchor: 0, pinned: false });
    },
    [onPanelChange, onPanelTarget],
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
      !popupInside.current &&
      !pinned &&
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
    setHoveredId(id);
    if (explicit && active === id) {
      setPinned((v) => !v);
      return;
    }
    if (!explicit && (pinned || requests.length)) return;
    if (explicit) setPinned(true);
    setActive(id);
    updateAnchor(id);
  }
  function leave() {
    pointerInside.current = false;
    cancelClose();
    setHoveredId(null);
    if (requests.length || pinned) return;
    closeTimer.current = setTimeout(() => {
      if (
        popupInside.current ||
        panelRef.current?.contains(document.activeElement)
      )
        return;
      setActive(null);
    }, s.dockHoverDelayMs);
  }
  function dismiss() {
    cancelClose();
    setDismissed((ids) => [...new Set([...ids, ...requests.map((r) => r.id)])]);
    setPinned(false);
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
    "--dock-size": `${size}px`,
    "--dock-gap": `${s.dockGap}px`,
    "--dock-padding": `${s.dockPadding}px`,
    "--dock-panel-width": `${s.dockPanelWidth}px`,
    "--dock-rail-width": `${size + s.dockPadding}px`,
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
        <div
          className="dock-drag-handle"
          title={t(s.dockLocked ? "unlockDockHint" : "dragDock")}
          onPointerDown={(e) => {
            if (e.button !== 0 || s.dockLocked) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            dragOrigin.current = { x: e.screenX, y: e.screenY };
            dismiss();
            onDrag?.("begin");
          }}
          onPointerMove={(e) => {
            if (dragOrigin.current)
              onDrag?.("move", {
                dx: e.screenX - dragOrigin.current.x,
                dy: e.screenY - dragOrigin.current.y,
              });
          }}
          onPointerUp={(e) => {
            if (!dragOrigin.current) return;
            dragOrigin.current = null;
            e.currentTarget.releasePointerCapture(e.pointerId);
            onDrag?.("end");
          }}
          onPointerCancel={() => {
            if (dragOrigin.current) {
              dragOrigin.current = null;
              onDrag?.("end");
            }
          }}
        >
          {s.dockLocked ? (
            <LockKeyhole size={13} />
          ) : (
            <GripVertical size={13} />
          )}
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
                magnified={hoveredId === agent.profile.id}
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
      {current && !detached && (
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
          <DockDetails
            current={current}
            s={s}
            requests={requests}
            sessions={data.sessions}
            drafts={drafts}
            onDraftChange={onDraftChange}
            t={t}
            busy={busy}
            now={now}
            pinned={pinned}
            onPin={() => setPinned((v) => !v)}
            onClose={dismiss}
            onDetails={onDetails}
            onRefresh={onRefresh}
            onSessions={onSessions}
            onRespond={onRespond}
          />
        </div>
      )}
    </div>
  );
}
