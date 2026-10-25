import { useState } from "react";
import { ArrowUp, ArrowDown, Check } from "lucide-react";
import type { Dashboard, Settings } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
export function SettingsPanel({
  data,
  t,
  onSave,
}: {
  data: Dashboard;
  t: T;
  onSave: (s: Settings) => Promise<void>;
}) {
  const [s, set] = useState(data.settings);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const update = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    set((v) => ({ ...v, [key]: value }));
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
  async function save() {
    setBusy(true);
    setError("");
    try {
      await onSave(s);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="settings-panel">
      <div className="page-heading">
        <div>
          <span className="eyebrow">PREFERENCES</span>
          <h2>{t("settings")}</h2>
        </div>
        <button className="primary-button" onClick={save} disabled={busy}>
          <Check size={15} />
          {t("save")}
        </button>
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
