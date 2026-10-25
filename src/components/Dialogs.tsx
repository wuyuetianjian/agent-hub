import { useState } from "react";
import { X, FolderOpen, Play, Check } from "lucide-react";
import { chooseDirectory } from "../api";
import type { AgentView, Profile } from "../types";
import type { T } from "../i18n";
function array(value: string) {
  const parsed: unknown = JSON.parse(value);
  if (!Array.isArray(parsed) || parsed.some((v) => typeof v !== "string"))
    throw new Error("Arguments must be a JSON string array");
  return parsed as string[];
}
export function ProfileDialog({
  agent,
  t,
  onClose,
  onSave,
  create = false,
}: {
  agent: AgentView;
  t: T;
  onClose: () => void;
  onSave: (p: Profile) => Promise<void>;
  create?: boolean;
}) {
  const [p, set] = useState({
    ...agent.profile,
    ...(create ? { id: "", label: agent.profile.label + " 2" } : {}),
  });
  const [args, setArgs] = useState(JSON.stringify(p.args));
  const [quotaArgs, setQuotaArgs] = useState(JSON.stringify(p.quotaArgs));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function save() {
    setBusy(true);
    setError("");
    try {
      await onSave({ ...p, args: array(args), quotaArgs: array(quotaArgs) });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="profile-title"
        className="modal"
      >
        <header>
          <div>
            <span className="eyebrow">{agent.provider.name}</span>
            <h2 id="profile-title">
              {create ? t("addProfile") : t("configure")}
            </h2>
          </div>
          <button className="icon-button" title={t("close")} onClick={onClose}>
            <X size={18} />
          </button>
        </header>
        <div className="modal-body">
          <label>
            {t("profileLabel")}
            <input
              autoFocus
              value={p.label}
              onChange={(e) => set({ ...p, label: e.target.value })}
            />
          </label>
          <p className="field-hint">{t("cliHint")}</p>
          <label>
            {t("cliPath")}
            <input
              value={p.executable}
              placeholder={
                agent.discovery.executable ||
                agent.provider.executables[0] ||
                ""
              }
              onChange={(e) => set({ ...p, executable: e.target.value })}
            />
          </label>
          <label>
            {t("cliArgs")}
            <input value={args} onChange={(e) => setArgs(e.target.value)} />
          </label>
          <label>
            {t("configDir")}
            <input
              value={p.configDir}
              onChange={(e) => set({ ...p, configDir: e.target.value })}
            />
          </label>
          <div className="form-divider" />
          <p className="field-hint">{t("collectorHint")}</p>
          <label>
            {t("quotaCommand")}
            <input
              value={p.quotaCommand}
              onChange={(e) => set({ ...p, quotaCommand: e.target.value })}
            />
          </label>
          <label>
            {t("quotaArgs")}
            <input
              value={quotaArgs}
              onChange={(e) => setQuotaArgs(e.target.value)}
            />
          </label>
          <label>
            {t("quotaFile")}
            <input
              value={p.quotaFile}
              onChange={(e) => set({ ...p, quotaFile: e.target.value })}
            />
          </label>
          {error && <p className="form-error">{error}</p>}
        </div>
        <footer>
          <button className="secondary-button" onClick={onClose}>
            {t("cancel")}
          </button>
          <button className="primary-button" disabled={busy} onClick={save}>
            <Check size={14} />
            {t("save")}
          </button>
        </footer>
      </section>
    </div>
  );
}
export function LaunchDialog({
  agents,
  initial,
  t,
  onClose,
  onLaunch,
}: {
  agents: AgentView[];
  initial?: string;
  t: T;
  onClose: () => void;
  onLaunch: (profileId: string, cwd: string, mode: string) => Promise<void>;
}) {
  const [profile, setProfile] = useState(
    initial || agents[0]?.profile.id || "",
  );
  const [cwd, setCwd] = useState("");
  const [mode, setMode] = useState("terminal");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const agent = agents.find((a) => a.profile.id === profile);
  async function launch() {
    setBusy(true);
    setError("");
    try {
      await onLaunch(profile, cwd, mode);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="modal-backdrop">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="launch-title"
        className="modal small"
      >
        <header>
          <h2 id="launch-title">{t("newSession")}</h2>
          <button className="icon-button" title={t("close")} onClick={onClose}>
            <X size={18} />
          </button>
        </header>
        <div className="modal-body">
          <label>
            {t("chooseAgent")}
            <select
              value={profile}
              onChange={(e) => {
                setProfile(e.target.value);
                setMode("terminal");
              }}
            >
              {agents.map((a) => (
                <option value={a.profile.id} key={a.profile.id}>
                  {a.provider.name} · {a.profile.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("project")}
            <span className="input-with-button">
              <input
                autoFocus
                value={cwd}
                onChange={(e) => setCwd(e.target.value)}
              />
              <button
                className="icon-button"
                title={t("selectFolder")}
                onClick={() =>
                  void chooseDirectory()
                    .then((path) => {
                      if (path) setCwd(path);
                    })
                    .catch((e) => setError(String(e)))
                }
              >
                <FolderOpen size={17} />
              </button>
            </span>
          </label>
          <label>
            {t("mode")}
            <select value={mode} onChange={(e) => setMode(e.target.value)}>
              <option value="terminal">{t("terminal")}</option>
              {agent?.provider.id === "codex" && (
                <option value="structured">
                  {t("structured")} · Codex app-server
                </option>
              )}
            </select>
          </label>
          {error && <p className="form-error">{error}</p>}
        </div>
        <footer>
          <button className="secondary-button" onClick={onClose}>
            {t("cancel")}
          </button>
          <button
            className="primary-button"
            disabled={busy || !cwd.trim()}
            onClick={launch}
          >
            <Play size={14} />
            {busy ? t("working") : t("launch")}
          </button>
        </footer>
      </section>
    </div>
  );
}
