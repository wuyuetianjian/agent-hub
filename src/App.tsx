import {
  useCallback,
  useEffect,
  useState,
  useRef,
  lazy,
  Suspense,
} from "react";
import {
  Orbit,
  Pin,
  PinOff,
  Expand,
  Minimize2,
  Minus,
  Settings2,
  RefreshCw,
  Plus,
  ArrowUpRight,
  LayoutGrid,
  TerminalSquare,
  ShieldCheck,
  ChevronRight,
  X,
  Power,
  ChevronDown,
  Search,
  CheckCircle2,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { call, isDesktop, onEvent, openUrl } from "./api";
import type {
  AgentView,
  Dashboard,
  Profile,
  SessionInfo,
  Settings,
} from "./types";
import { translator } from "./i18n";
import { ExternalSession } from "./components/ExternalSession";
import { AgentDock } from "./components/AgentDock";
import { DockDetails } from "./components/DockDetails";
import { requestsForProfile } from "./lib/dock";
import { dockAgents as getDockAgents } from "./lib/dock";
import { AgentIcon } from "./components/AgentIcon";
import { QuotaCard } from "./components/QuotaCard";
import { AgentDetail } from "./components/AgentDetail";
import { ApprovalCard, type ApprovalDraft } from "./components/ApprovalCard";
import { SettingsPanel } from "./components/SettingsPanel";
import { ProfileDialog, LaunchDialog } from "./components/Dialogs";
import { summaryWindow } from "./lib/summary";
const Terminal = lazy(() =>
  import("./components/Terminal").then((m) => ({ default: m.Terminal })),
);
const workspaceWindow = new URLSearchParams(window.location.search).has(
  "workspace",
);
const panelWindow = new URLSearchParams(window.location.search).has("panel");
type Page = "overview" | "agents" | "sessions" | "approvals" | "settings";
export default function App() {
  const [data, setData] = useState<Dashboard | null>(null);
  const [expanded, setExpanded] = useState(workspaceWindow);
  const [approvalDrafts, setApprovalDrafts] = useState<
    Record<string, ApprovalDraft>
  >({});
  const updateApprovalDraft = (id: string, draft: ApprovalDraft) =>
    setApprovalDrafts((current) => ({ ...current, [id]: draft }));
  const [, setDockOpen] = useState(false);
  const [panelTarget, setPanelTarget] = useState<{
    profileId: string;
    pinned: boolean;
  } | null>(null);
  const [panelPresence, setPanelPresence] = useState<{
    inside: boolean;
    dismiss?: boolean;
    togglePin?: boolean;
    sequence: number;
  }>({ inside: false, sequence: 0 });
  const panelRoot = useRef<HTMLDivElement>(null);
  const dragQueue = useRef(Promise.resolve());
  const panelRequestQueue = useRef(Promise.resolve());
  const setPanel = useCallback(
    (target: { profileId: string | null; anchor: number; pinned: boolean }) => {
      if (!isDesktop || panelWindow || workspaceWindow) return;
      panelRequestQueue.current = panelRequestQueue.current
        .catch(() => {})
        .then(() => call<void>("set_dock_panel", target))
        .catch((e) => setError(String(e)));
    },
    [],
  );
  const [page, setPage] = useState<Page>("overview");
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [configure, setConfigure] = useState<{
    id: string;
    create?: boolean;
  } | null>(null);
  const [launch, setLaunch] = useState<string | null>(null);
  const [session, setSession] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [menu, setMenu] = useState(false);
  useEffect(() => {
    if (!data) return;
    const liveIds = new Set(data.interactions.map((request) => request.id));
    setApprovalDrafts((current) => {
      if (Object.keys(current).every((id) => liveIds.has(id))) return current;
      return Object.fromEntries(
        Object.entries(current).filter(([id]) => liveIds.has(id)),
      );
    });
  }, [data?.interactions]);
  const load = useCallback(async () => {
    try {
      setData(await call<Dashboard>("get_dashboard"));
    } catch (e) {
      setError(String(e));
    }
  }, []);
  useEffect(() => {
    void load();
    let disposed = false;
    let off: (() => void) | undefined;
    void onEvent((e) => {
      if (e.kind === "dashboard") void load();
    }).then((fn) => {
      if (disposed) fn();
      else off = fn;
    });
    const timer = setInterval(() => void load(), 15000);
    return () => {
      disposed = true;
      off?.();
      clearInterval(timer);
    };
  }, [load]);
  useEffect(() => {
    if (!isDesktop || !workspaceWindow) return;
    let disposed = false;
    let off: (() => void) | undefined;
    void listen<{ page: Page; profileId?: string; sessionId?: string }>(
      "orbit:navigate",
      (event) => {
        setPage(event.payload.page);
        setSelected(event.payload.profileId ?? null);
        setSession(event.payload.sessionId ?? null);
        setExpanded(true);
      },
    ).then((fn) => {
      if (disposed) fn();
      else off = fn;
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, []);
  useEffect(() => {
    if (!isDesktop || workspaceWindow) return;
    let disposed = false;
    let off: (() => void) | undefined;
    const topic = panelWindow ? "orbit:dock-panel" : "orbit:panel-presence";
    void listen<any>(topic, (event) => {
      if (panelWindow) setPanelTarget(event.payload);
      else
        setPanelPresence((current) => ({
          ...event.payload,
          sequence: current.sequence + 1,
        }));
    }).then((fn) => {
      if (disposed) fn();
      else {
        off = fn;
        if (panelWindow)
          void call<{ profileId: string; pinned: boolean } | null>(
            "get_dock_panel",
          ).then(setPanelTarget);
      }
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, []);
  useEffect(() => {
    if (!panelWindow || !panelRoot.current || !panelTarget) return;
    const observer = new ResizeObserver((entries) => {
      const height = Math.ceil(
        entries[0].target.getBoundingClientRect().height,
      );
      if (height > 0)
        void call("resize_dock_panel", {
          height,
          profileId: panelTarget.profileId,
        }).catch((e) => setError(String(e)));
    });
    observer.observe(panelRoot.current);
    return () => observer.disconnect();
  }, [panelTarget?.profileId, Boolean(data)]);
  const mode = data?.settings.windowMode;
  const compact = data?.settings.compact;
  const island = mode === "island" && !expanded;
  const dock = !expanded && mode !== "island";
  const dockCount = data
    ? getDockAgents(
        data.agents,
        data.settings.visibleProviders,
        data.sessions,
        data.interactions,
      ).length
    : 0;
  const nativeView = expanded
    ? "workspace"
    : island
      ? selected
        ? "detail"
        : "widget"
      : "dock";
  useEffect(() => {
    if (data && !workspaceWindow && !panelWindow)
      void call("set_window_view", {
        view: nativeView,
      }).catch((e) => setError(String(e)));
  }, [
    nativeView,
    data?.settings.dockHoverSize,
    mode,
    compact,
    dockCount,
    data?.settings.dockSide,
    data?.settings.dockSize,
    data?.settings.dockGap,
    Boolean(data),
  ]);
  useEffect(() => {
    const fn = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setConfigure(null);
        setLaunch(null);
        setSelected(null);
        setMenu(false);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        if (isDesktop && !workspaceWindow)
          void call("open_workspace", { page: "settings" });
        else {
          setExpanded(true);
          setPage("settings");
        }
      }
    };
    window.addEventListener("keydown", fn);
    return () => window.removeEventListener("keydown", fn);
  }, []);
  async function refresh(id: string) {
    setBusy((v) => [...v, id]);
    try {
      await call("refresh_profile", { profileId: id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy((v) => v.filter((x) => x !== id));
    }
  }
  async function saveSettings(s: Settings) {
    await call("save_settings", { settings: s });
    await load();
  }
  async function saveProfile(profile: Profile) {
    await call("save_profile", { profile });
    await load();
  }
  async function start(profileId: string, cwd: string, mode: string) {
    const created = await call<SessionInfo>("start_session", {
      profileId,
      cwd,
      mode,
    });
    if (isDesktop && !workspaceWindow) {
      await call("open_workspace", { page: "sessions", sessionId: created.id });
    } else {
      setSession(created.id);
      setExpanded(true);
      setSelected(null);
      setPage("sessions");
    }
    await load();
  }
  if (!data)
    return (
      <div className="startup">
        <Orbit className="spin" size={28} />
        <span>Agent Orbit</span>
        {error && <p role="alert">{error}</p>}
      </div>
    );
  const t = translator(data.settings.locale);
  const active = data.agents
    .filter((a) => data.settings.visibleProviders.includes(a.provider.id))
    .sort(
      (a, b) =>
        data.settings.visibleProviders.indexOf(a.provider.id) -
        data.settings.visibleProviders.indexOf(b.provider.id),
    );
  const chosen = data.agents.find((a) => a.profile.id === selected);
  const currentSession =
    data.sessions.find((s) => s.id === session) ||
    data.sessions.find((s) => s.profileId === selected) ||
    data.sessions.find((s) => s.status === "running") ||
    data.sessions[0];
  const configured = data.agents.find((a) => a.profile.id === configure?.id);
  const count = data.agents.filter((a) => a.discovery.installed).length;
  const filtered = data.agents.filter((a) =>
    `${a.provider.name} ${a.profile.label}`
      .toLowerCase()
      .includes(search.toLowerCase()),
  );
  const theme =
    data.settings.theme === "system"
      ? matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : data.settings.theme;
  function openPage(p: Page, profileId?: string, sessionId?: string) {
    if (isDesktop && !workspaceWindow) {
      void call("open_workspace", { page: p, profileId, sessionId }).catch(
        (e) => setError(String(e)),
      );
      return;
    }
    if (sessionId !== undefined) setSession(sessionId);
    setExpanded(true);
    setPage(p);
    setSelected(profileId ?? null);
  }
  const detail = (agent: AgentView) => (
    <AgentDetail
      agent={agent}
      settings={data!.settings}
      t={t}
      busy={busy.includes(agent.profile.id)}
      sessions={data.sessions.filter((s) => s.profileId === agent.profile.id)}
      onSession={(id) => {
        openPage("sessions", undefined, id);
      }}
      onBack={() => setSelected(null)}
      onRefresh={() => void refresh(agent.profile.id)}
      onConfigure={() => setConfigure({ id: agent.profile.id })}
      onLaunch={() => setLaunch(agent.profile.id)}
      onUsage={() =>
        void openUrl(agent.provider.usageUrl).catch((e) => setError(String(e)))
      }
    />
  );
  const approvals = (
    <div className="approvals-list">
      {data.interactions.length ? (
        data.interactions.map((r) => (
          <ApprovalCard
            key={r.id}
            request={r}
            draft={approvalDrafts[r.id]}
            onDraftChange={(draft) => updateApprovalDraft(r.id, draft)}
            provider={
              data.agents.find((a) => a.provider.id === r.providerId)?.provider
            }
            t={t}
            onRespond={async (reply) => {
              await call("respond", { requestId: r.id, reply });
              await load();
            }}
          />
        ))
      ) : (
        <div className="empty-state">
          <CheckCircle2 size={30} strokeWidth={1.2} />
          <h3>{t("noApprovals")}</h3>
          <p>{t("noApprovalsHint")}</p>
        </div>
      )}
    </div>
  );
  if (panelWindow) {
    const current = data.agents.find(
      (a) => a.profile.id === panelTarget?.profileId,
    );
    const now = Date.now() / 1000;
    const presence = (inside: boolean, dismiss = false, togglePin = false) => {
      void call("dock_panel_pointer", { inside, dismiss, togglePin });
    };
    return (
      <div
        data-theme={theme}
        className="detached-panel-root"
        ref={panelRoot}
        onMouseEnter={() => presence(true)}
        onMouseLeave={() => presence(false)}
        onFocusCapture={() => presence(true)}
        onBlurCapture={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget))
            presence(false);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            presence(false, true);
          }
        }}
      >
        {current && (
          <div
            className="dock-popover detached-popover"
            style={{ maxHeight: data.settings.dockPopupHeight }}
          >
            <DockDetails
              current={current}
              s={data.settings}
              requests={requestsForProfile(
                current.profile.id,
                data.sessions,
                data.interactions.filter((r) => r.expiresAt > now),
              )}
              sessions={data.sessions}
              drafts={approvalDrafts}
              onDraftChange={updateApprovalDraft}
              t={t}
              busy={busy}
              now={now}
              pinned={panelTarget?.pinned}
              onPin={() => presence(true, false, true)}
              onClose={() => presence(false, true)}
              onDetails={(id) => openPage("agents", id)}
              onRefresh={(id) => void refresh(id)}
              onSessions={(id) =>
                openPage(
                  "sessions",
                  id,
                  data.sessions.find((session) => session.profileId === id)?.id,
                )
              }
              onRespond={async (id, reply) => {
                await call("respond", { requestId: id, reply });
                await load();
              }}
            />
          </div>
        )}
      </div>
    );
  }
  return (
    <div
      className={`app-frame ${expanded ? "expanded-frame" : ""} ${dock ? "dock-frame" : ""} ${!isDesktop ? "preview-frame" : ""}`}
      data-theme={theme}
    >
      {dock ? (
        <AgentDock
          drafts={approvalDrafts}
          onDraftChange={updateApprovalDraft}
          data={data}
          t={t}
          busy={busy}
          onPanelChange={setDockOpen}
          detached={isDesktop}
          onPanelTarget={setPanel}
          panelPresence={panelPresence}
          onDrag={(phase, delta) => {
            const command =
              phase === "begin"
                ? "drag_dock"
                : phase === "move"
                  ? "move_dock"
                  : "finish_dock_drag";
            dragQueue.current = dragQueue.current
              .catch(() => {})
              .then(() => call<void>(command, delta))
              .catch((e) => setError(String(e)));
          }}
          onRefresh={(id) => void refresh(id)}
          onDetails={(id) => openPage("agents", id)}
          onSettings={() => openPage("settings")}
          onSessions={(id) => {
            openPage(
              "sessions",
              id,
              data.sessions.find((s) => s.profileId === id)?.id,
            );
          }}
          onRespond={async (id, reply) => {
            await call("respond", { requestId: id, reply });
            await load();
          }}
        />
      ) : (
        <div
          className={`shell ${expanded ? "workspace" : "widget"} ${island ? "island" : ""}`}
        >
          {island ? (
            <>
              <div className="island-notch" data-tauri-drag-region />
              <div className="island-bar">
                <button
                  className="island-brand"
                  title={t("expand")}
                  onClick={() => openPage("overview")}
                >
                  <Orbit size={20} />
                </button>
                <div className="island-agents">
                  {active.slice(0, 7).map((a) => {
                    const w = summaryWindow(
                      a.snapshot.windows,
                      data.settings.summaryPolicy,
                    );
                    return (
                      <button
                        key={a.profile.id}
                        title={`${a.provider.name} · ${w?.label || t("unknown")}`}
                        className={a.profile.id === selected ? "active" : ""}
                        onClick={() =>
                          setSelected(
                            selected === a.profile.id ? null : a.profile.id,
                          )
                        }
                      >
                        <AgentIcon provider={a.provider} size={27} />
                        <span>
                          {w?.remainingPercent == null
                            ? "—"
                            : `${Math.round(w.remainingPercent)}%`}
                        </span>
                      </button>
                    );
                  })}
                </div>
                <button
                  className="island-more"
                  title={t(data.interactions.length ? "approvals" : "settings")}
                  onClick={() =>
                    openPage(
                      data.interactions.length ? "approvals" : "settings",
                    )
                  }
                >
                  {data.interactions.length ? (
                    <span className="approval-dot">
                      {data.interactions.length}
                    </span>
                  ) : (
                    <Settings2 size={16} />
                  )}
                </button>
                {chosen && (
                  <button
                    className="icon-button"
                    title={t("collapse")}
                    onClick={() => setSelected(null)}
                  >
                    <ChevronDown size={14} />
                  </button>
                )}
              </div>
              {chosen && <div className="island-detail">{detail(chosen)}</div>}
            </>
          ) : (
            <>
              <header className="titlebar" data-tauri-drag-region>
                <div className="brand" data-tauri-drag-region>
                  <span className="brand-symbol">
                    <Orbit size={20} />
                  </span>
                  <strong>Agent Orbit</strong>
                  <span className="version">LOCAL</span>
                </div>
                <div className="window-actions">
                  <button
                    className={`icon-button ${data.settings.alwaysOnTop ? "on" : ""}`}
                    title={data.settings.alwaysOnTop ? t("pinned") : t("pin")}
                    onClick={() =>
                      void saveSettings({
                        ...data.settings,
                        alwaysOnTop: !data.settings.alwaysOnTop,
                      }).catch((e) => setError(String(e)))
                    }
                  >
                    {data.settings.alwaysOnTop ? (
                      <Pin size={14} />
                    ) : (
                      <PinOff size={14} />
                    )}
                  </button>
                  <button
                    className="icon-button"
                    title={expanded ? t("collapse") : t("expand")}
                    onClick={() => {
                      if (workspaceWindow) {
                        void call("hide_window");
                        return;
                      }
                      setExpanded(!expanded);
                      setSelected(null);
                    }}
                  >
                    {expanded ? <Minimize2 size={15} /> : <Expand size={15} />}
                  </button>
                  <button
                    className="icon-button"
                    title={t("hide")}
                    onClick={() =>
                      void call("hide_window").catch((e) => setError(String(e)))
                    }
                  >
                    <Minus size={15} />
                  </button>
                  <button
                    className="icon-button"
                    title={t("settings")}
                    onClick={() => setMenu(!menu)}
                  >
                    <ChevronDown size={13} />
                  </button>
                </div>
                {menu && (
                  <div className="window-menu">
                    <button
                      onClick={() => {
                        openPage("settings");
                        setMenu(false);
                      }}
                    >
                      <Settings2 size={14} />
                      {t("settings")}
                    </button>
                    <button
                      onClick={() =>
                        void call("quit_app").catch((e) => setError(String(e)))
                      }
                    >
                      <Power size={14} />
                      {t("quit")}
                    </button>
                  </div>
                )}
              </header>
              {expanded ? (
                <div className="workspace-body">
                  <aside className="sidebar">
                    <div className="nav-label">WORKSPACE</div>
                    {(
                      [
                        ["overview", LayoutGrid],
                        ["agents", Orbit],
                        ["sessions", TerminalSquare],
                        ["approvals", ShieldCheck],
                      ] as const
                    ).map(([key, Icon]) => (
                      <button
                        className={page === key ? "active" : ""}
                        key={key}
                        onClick={() => {
                          setPage(key);
                          setSelected(null);
                        }}
                      >
                        <Icon size={17} />
                        {t(key)}
                        {key === "approvals" &&
                          data.interactions.length > 0 && (
                            <span className="nav-count">
                              {data.interactions.length}
                            </span>
                          )}
                      </button>
                    ))}
                    <div className="sidebar-bottom">
                      <span>
                        <i className="status-dot" />
                        {t("localOnly")}
                      </span>
                      <button
                        className={page === "settings" ? "active" : ""}
                        onClick={() => {
                          setPage("settings");
                          setSelected(null);
                        }}
                      >
                        <Settings2 size={17} />
                        {t("settings")}
                      </button>
                    </div>
                  </aside>
                  <main
                    className={`workspace-main ${page === "sessions" ? "session-main" : ""}`}
                  >
                    {chosen ? (
                      detail(chosen)
                    ) : page === "settings" ? (
                      <SettingsPanel
                        onResetDock={() => call<void>("reset_dock_position")}
                        key={data.settings.locale}
                        data={data}
                        t={t}
                        onSave={saveSettings}
                      />
                    ) : page === "sessions" ? (
                      <>
                        <div className="page-heading">
                          <div>
                            <span className="eyebrow">
                              YOUR WORK, IN MOTION
                            </span>
                            <h2>{t("sessions")}</h2>
                          </div>
                          <button
                            className="primary-button"
                            onClick={() => setLaunch("")}
                          >
                            <Plus size={14} />
                            {t("newSession")}
                          </button>
                        </div>
                        {data.sessions.length ? (
                          <>
                            <div className="session-tabs">
                              {data.sessions.map((s) => (
                                <button
                                  title={s.title || s.cwd}
                                  className={
                                    currentSession?.id === s.id ? "active" : ""
                                  }
                                  key={s.id}
                                  onClick={() => setSession(s.id)}
                                >
                                  <span
                                    className={`status-dot ${s.status !== "running" ? "muted" : ""}`}
                                  />
                                  {
                                    data.agents.find(
                                      (a) => a.profile.id === s.profileId,
                                    )?.provider.name
                                  }
                                  <small>
                                    {s.readOnly
                                      ? t("externalSession")
                                      : s.cwd
                                          .split(/[\\/]/)
                                          .filter(Boolean)
                                          .at(-1)}
                                  </small>
                                </button>
                              ))}
                            </div>
                            {currentSession && (
                              <>
                                <div className="terminal-toolbar">
                                  <span>{currentSession.cwd}</span>
                                  <button
                                    className="text-button"
                                    disabled={
                                      currentSession.status !== "running"
                                    }
                                    onClick={() =>
                                      void call("stop_session", {
                                        sessionId: currentSession.id,
                                      })
                                        .then(load)
                                        .catch((e) => setError(String(e)))
                                    }
                                  >
                                    {t("stop")}
                                  </button>
                                </div>
                                {currentSession.readOnly ? (
                                  <ExternalSession
                                    session={currentSession}
                                    t={t}
                                    locale={data.settings.locale}
                                  />
                                ) : (
                                  <Suspense
                                    fallback={
                                      <div className="empty-state">
                                        {t("loading")}
                                      </div>
                                    }
                                  >
                                    <Terminal
                                      session={currentSession}
                                      t={t}
                                      onError={setError}
                                    />
                                  </Suspense>
                                )}
                              </>
                            )}
                          </>
                        ) : (
                          <div className="empty-state tall">
                            <TerminalSquare size={36} strokeWidth={1.2} />
                            <h3>{t("noSessions")}</h3>
                            <p>{t("sessionHint")}</p>
                          </div>
                        )}
                        {data.interactions.length > 0 && approvals}
                      </>
                    ) : page === "approvals" ? (
                      <>
                        <div className="page-heading">
                          <div>
                            <span className="eyebrow">HUMAN IN THE LOOP</span>
                            <h2>{t("inbox")}</h2>
                          </div>
                          <span className="count-pill">
                            {data.interactions.length}
                          </span>
                        </div>
                        {approvals}
                      </>
                    ) : (
                      <>
                        <div className="page-heading">
                          <div>
                            <span className="eyebrow">
                              A LITTLE CLARITY, ALWAYS NEARBY
                            </span>
                            <h2>
                              {page === "agents"
                                ? t("allAgents")
                                : t("overview")}
                              <span className="heading-dot">.</span>
                            </h2>
                            <p>
                              {data.agents.length} agents · {count}{" "}
                              {t("connected")}
                            </p>
                          </div>
                          <button
                            className="secondary-button"
                            disabled={busy.length > 0}
                            onClick={() =>
                              void Promise.all(
                                data.agents
                                  .filter(
                                    (a) =>
                                      a.discovery.installed ||
                                      a.profile.quotaCommand ||
                                      a.profile.quotaFile,
                                  )
                                  .map((a) => refresh(a.profile.id)),
                              )
                            }
                          >
                            <RefreshCw
                              size={14}
                              className={busy.length ? "spin" : ""}
                            />
                            {t("refreshAll")}
                          </button>
                        </div>
                        {page === "agents" && (
                          <label className="search">
                            <Search size={15} />
                            <input
                              placeholder={t("chooseAgent")}
                              value={search}
                              onChange={(e) => setSearch(e.target.value)}
                            />
                          </label>
                        )}
                        <div className="agents-grid">
                          {(page === "agents" ? filtered : active).map((a) => (
                            <div key={a.profile.id}>
                              <QuotaCard
                                agent={a}
                                settings={data.settings}
                                t={t}
                                selected={false}
                                onClick={() => setSelected(a.profile.id)}
                              />
                              {page === "agents" && (
                                <div className="agent-grid-actions">
                                  <button
                                    onClick={() =>
                                      setConfigure({ id: a.profile.id })
                                    }
                                  >
                                    {t("configure")}
                                  </button>
                                  <button
                                    title={t("addProfile")}
                                    onClick={() =>
                                      setConfigure({
                                        id: a.profile.id,
                                        create: true,
                                      })
                                    }
                                  >
                                    <Plus size={12} />
                                  </button>
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                        {page === "overview" && (
                          <>
                            <div className="section-heading">
                              <h3>{t("inbox")}</h3>
                              <button
                                className="text-button"
                                onClick={() => openPage("approvals")}
                              >
                                {t("viewAll")}
                                <ArrowUpRight size={13} />
                              </button>
                            </div>
                            {approvals}
                          </>
                        )}
                      </>
                    )}
                  </main>
                </div>
              ) : chosen ? (
                detail(chosen)
              ) : (
                <div className="widget-body">
                  <div className="widget-heading">
                    <div>
                      <span className="eyebrow">YOUR AGENTS, AT A GLANCE</span>
                      <h1>
                        {t("quota")}
                        <span className="heading-dot">.</span>
                      </h1>
                    </div>
                    <button
                      className="refresh-round"
                      title={t("refreshAll")}
                      disabled={busy.length > 0}
                      onClick={() =>
                        void Promise.all(
                          active.map((a) => refresh(a.profile.id)),
                        )
                      }
                    >
                      <RefreshCw
                        size={16}
                        className={busy.length ? "spin" : ""}
                      />
                    </button>
                  </div>
                  <div className="widget-summary">
                    <span>
                      <i className="status-dot" />
                      {count} {t("connected")}
                    </span>
                    <span>
                      {t("autoRefresh")} · {data.settings.refreshSeconds}s
                    </span>
                  </div>
                  <div className="widget-agent-list">
                    {active.length ? (
                      active.map((a) => (
                        <QuotaCard
                          key={a.profile.id}
                          agent={a}
                          settings={data.settings}
                          t={t}
                          selected={false}
                          onClick={() => setSelected(a.profile.id)}
                        />
                      ))
                    ) : (
                      <button
                        className="full-button"
                        onClick={() => openPage("settings")}
                      >
                        {t("noVisible")}
                      </button>
                    )}
                  </div>
                  <button
                    className={`inbox-strip ${data.interactions.length ? "has-pending" : ""}`}
                    onClick={() => openPage("approvals")}
                  >
                    <ShieldCheck size={17} />
                    <span>
                      {data.interactions.length
                        ? `${data.interactions.length} ${t("pending")}`
                        : t("noApprovals")}
                    </span>
                    <ChevronRight size={14} />
                  </button>
                  <div className="widget-footer">
                    <button onClick={() => openPage("agents")}>
                      <LayoutGrid size={14} />
                      {t("allAgents")}
                      <span>{data.agents.length}</span>
                    </button>
                    <button onClick={() => openPage("sessions")}>
                      <TerminalSquare size={14} />
                      {t("sessions")}
                      <span>{data.sessions.length}</span>
                    </button>
                    <button onClick={() => setLaunch("")}>
                      <Plus size={14} />
                      {t("newSession")}
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
          {!isDesktop && (!island || chosen) && (
            <div className="preview-note">{t("preview")}</div>
          )}
        </div>
      )}
      {error && (
        <div className="toast" role="alert">
          <span>{error}</span>
          <button
            className="icon-button"
            title={t("close")}
            onClick={() => setError("")}
          >
            <X size={14} />
          </button>
        </div>
      )}
      {configured && (
        <ProfileDialog
          agent={configured}
          create={configure?.create}
          t={t}
          onClose={() => setConfigure(null)}
          onSave={saveProfile}
        />
      )}{" "}
      {launch !== null && (
        <LaunchDialog
          agents={data.agents}
          initial={launch || undefined}
          t={t}
          onClose={() => setLaunch(null)}
          onLaunch={start}
        />
      )}
    </div>
  );
}
