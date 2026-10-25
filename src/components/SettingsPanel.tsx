import { useState, useRef, useEffect } from "react";
import { ArrowUp, ArrowDown } from "lucide-react";
import type { Dashboard, Settings } from "../types";
import type { T } from "../i18n";
import { QuotaOrb } from "./QuotaOrb";
import { AgentIcon } from "./AgentIcon";
export function SettingsPanel({
  data,
  t,
  onSave,
  onResetDock,
}: {
  data: Dashboard;
  t: T;
  onSave: (s: Settings) => Promise<void>;
  onResetDock?: () => Promise<void>;
}) {
  const [s, set] = useState(data.settings);
  const [error, setError] = useState("");
  const latest = useRef(s);
  const pending = useRef<Settings | null>(null);
  const saving = useRef(false);
  useEffect(() => {
    if (saving.current || pending.current) return;
    latest.current = data.settings;
    set(data.settings);
  }, [data.settings]);
  async function flush() {
    if (saving.current) return;
    saving.current = true;
    try {
      while (pending.current) {
        const next = pending.current;
        pending.current = null;
        await onSave(next);
      }
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      saving.current = false;
    }
  }
  const apply = (next: Settings) => {
    latest.current = next;
    set(next);
    pending.current = next;
    void flush();
  };
  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    let next = { ...latest.current, [key]: value };
    if (key === "dockSize")
      next = {
        ...next,
        dockHoverSize: Math.max(next.dockHoverSize, next.dockSize),
      };
    apply(next);
  };
  const providers = [...data.agents.map((a) => a.provider)]
    .filter((p, i, all) => all.findIndex((v) => v.id === p.id) === i)
    .sort((a, b) => {
      const ai = s.visibleProviders.indexOf(a.id),
        bi = s.visibleProviders.indexOf(b.id);
      return (ai < 0 ? 999 : ai) - (bi < 0 ? 999 : bi);
    });
  function move(id: string, by: number) {
    const next = [...s.visibleProviders];
    const i = next.indexOf(id);
    if (i < 0 || i + by < 0 || i + by >= next.length) return;
    [next[i], next[i + by]] = [next[i + by], next[i]];
    update("visibleProviders", next);
  }
  return (
    <section className="settings-panel">
      <div className="page-heading">
        <div>
          <span className="eyebrow">PREFERENCES</span>
          <h2>{t("settings")}</h2>
        </div>
        <span className="field-hint">{t("autoSave")}</span>
      </div>
      {error && <p className="form-error">{error}</p>}
      <div className="settings-grid">
        <div className="settings-group">
          <h3>{t("appearance")}</h3>
          <label>
            {t("appearance")}
            <select
              value={s.theme}
              onChange={(e) =>
                update("theme", e.target.value as Settings["theme"])
              }
            >
              <option value="dark">{t("dark")}</option>
              <option value="light">{t("light")}</option>
              <option value="system">{t("system")}</option>
            </select>
          </label>
          <label>
            {t("language")}
            <select
              value={s.locale}
              onChange={(e) =>
                update("locale", e.target.value as Settings["locale"])
              }
            >
              <option value="zh">中文</option>
              <option value="en">English</option>
            </select>
          </label>
          <label>
            {t("displayMode")}
            <select
              value={s.windowMode}
              onChange={(e) =>
                update("windowMode", e.target.value as Settings["windowMode"])
              }
            >
              <option value="floating">{t("floating")}</option>
              {data.platform === "macos" && (
                <option value="island">{t("island")}</option>
              )}
            </select>
          </label>
          {s.windowMode === "floating" && (
            <label>
              {t("dockSide")}
              <select
                value={s.dockSide}
                onChange={(e) =>
                  update("dockSide", e.target.value as Settings["dockSide"])
                }
              >
                <option value="right">{t("right")}</option>
                <option value="left">{t("left")}</option>
              </select>
            </label>
          )}
          {s.windowMode === "floating" && (
            <>
              <div
                className="dock-size-presets"
                role="group"
                aria-label={t("dockSize")}
              >
                {s.dockPresets.map((preset) => (
                  <button
                    key={preset.id}
                    className={
                      s.dockSize === preset.size &&
                      s.dockHoverSize === preset.hoverSize
                        ? "primary-button"
                        : "secondary-button"
                    }
                    onClick={() => {
                      apply({
                        ...latest.current,
                        dockSize: preset.size,
                        dockHoverSize: preset.hoverSize,
                      });
                    }}
                  >
                    {t(
                      (["small", "medium", "large"].includes(preset.id)
                        ? preset.id
                        : "medium") as "small" | "medium" | "large",
                    )}
                  </button>
                ))}
              </div>
              {data.agents[0] && (
                <div
                  className="dock-size-preview"
                  style={
                    {
                      "--dock-size": `${s.dockHoverSize}px`,
                    } as React.CSSProperties
                  }
                >
                  <div>
                    <QuotaOrb
                      agent={data.agents[0]}
                      settings={s}
                      active={false}
                      onHover={() => {}}
                      onLeave={() => {}}
                      onSelect={() => {}}
                    />
                    <span>{t("dockSize")}</span>
                  </div>
                  <div>
                    <QuotaOrb
                      agent={data.agents[0]}
                      settings={s}
                      active={false}
                      magnified
                      onHover={() => {}}
                      onLeave={() => {}}
                      onSelect={() => {}}
                    />
                    <span>{t("dockHoverSize")}</span>
                  </div>
                </div>
              )}
              <label>
                {t("dockSize")} <output>{s.dockSize}px</output>
                <input
                  aria-label={t("dockSize")}
                  type="range"
                  min={24}
                  max={96}
                  step={1}
                  value={s.dockSize}
                  onChange={(e) => update("dockSize", Number(e.target.value))}
                />
              </label>
              <label>
                {t("dockHoverSize")} <output>{s.dockHoverSize}px</output>
                <input
                  aria-label={t("dockHoverSize")}
                  type="range"
                  min={s.dockSize}
                  max={120}
                  step={1}
                  value={s.dockHoverSize}
                  onChange={(e) =>
                    update("dockHoverSize", Number(e.target.value))
                  }
                />
              </label>
              <label className="switch-label">
                {t("dockLocked")}
                <input
                  type="checkbox"
                  checked={s.dockLocked}
                  onChange={(e) => update("dockLocked", e.target.checked)}
                />
              </label>
              <label>
                {t("dockEdgeInset")} <output>{s.dockEdgeInset}px</output>
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={s.dockEdgeInset}
                  onChange={(e) =>
                    update("dockEdgeInset", Number(e.target.value))
                  }
                />
              </label>
              {onResetDock && (
                <button
                  className="secondary-button"
                  onClick={() =>
                    void onResetDock().catch((e) => setError(String(e)))
                  }
                >
                  {t("resetDockPosition")}
                </button>
              )}
              <p className="field-hint">{t("dockLiveHint")}</p>
            </>
          )}
          {data.platform === "macos" && (
            <p className="field-hint">{t("islandHint")}</p>
          )}
          <label className="switch-label">
            {t("alwaysOnTop")}
            <input
              type="checkbox"
              checked={s.alwaysOnTop}
              onChange={(e) => update("alwaysOnTop", e.target.checked)}
            />
          </label>
          <label className="switch-label">
            {t("startAtLogin")}
            <input
              type="checkbox"
              checked={s.startAtLogin}
              onChange={(e) => update("startAtLogin", e.target.checked)}
            />
          </label>
        </div>
        <div className="settings-group">
          <h3>{t("quota")}</h3>
          <label>
            {t("summaryPolicy")}
            <select
              value={s.summaryPolicy}
              onChange={(e) =>
                update(
                  "summaryPolicy",
                  e.target.value as Settings["summaryPolicy"],
                )
              }
            >
              <option value="shortest">{t("shortest")}</option>
              <option value="lowest">{t("lowest")}</option>
            </select>
          </label>
          <label>
            {t("refreshInterval")}
            <input
              type="number"
              min={15}
              max={86400}
              value={s.refreshSeconds}
              onChange={(e) => update("refreshSeconds", Number(e.target.value))}
            />
          </label>
          <label>
            {t("alertThreshold")}
            <input
              type="number"
              min={0}
              max={100}
              value={s.warningPercent}
              onChange={(e) => update("warningPercent", Number(e.target.value))}
            />
          </label>
          <div className="local-note">
            <span className="status-dot" />
            {t("localOnly")}
            <small>{t("credentials")}</small>
          </div>
        </div>
      </div>
      <div className="settings-group">
        <h3>{t("visibility")}</h3>
        <div className="provider-checks">
          {providers.map((p) => (
            <div key={p.id} className="provider-check">
              <label>
                <input
                  type="checkbox"
                  checked={s.visibleProviders.includes(p.id)}
                  onChange={(e) =>
                    update(
                      "visibleProviders",
                      e.target.checked
                        ? [...s.visibleProviders, p.id]
                        : s.visibleProviders.filter((v) => v !== p.id),
                    )
                  }
                />
                <AgentIcon provider={p} size={24} />
                <span>{p.name}</span>
              </label>
              {s.visibleProviders.includes(p.id) && (
                <span>
                  <button
                    className="icon-button"
                    title="Move up"
                    onClick={() => move(p.id, -1)}
                  >
                    <ArrowUp size={13} />
                  </button>
                  <button
                    className="icon-button"
                    title="Move down"
                    onClick={() => move(p.id, 1)}
                  >
                    <ArrowDown size={13} />
                  </button>
                </span>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
