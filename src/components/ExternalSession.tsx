import { useState } from "react";
import { History, FolderOpen, Clock3, PlugZap } from "lucide-react";
import type { SessionInfo } from "../types";
import type { T } from "../i18n";
export function ExternalSession({
  session,
  t,
  locale,
  onConnect,
  coverage,
  onDisableAll,
}: {
  session: SessionInfo;
  t: T;
  locale: string;
  onConnect?: () => Promise<{ installCommand: string }>;
  coverage?: { profileId: string; installed: boolean };
  onDisableAll?: () => Promise<void>;
}) {
  const [setup, setSetup] = useState<{ installCommand: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const hook = session.mode === "hook";
  const connected = session.status === "hook-connected";
  async function connect() {
    if (!onConnect) return;
    setBusy(true);
    setError("");
    try {
      setSetup(await onConnect());
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="external-session" aria-label={t("externalSession")}>
      <div className="external-session-heading">
        <History size={23} />
        <div>
          <span className="status-chip">
            {t(hook ? "codexHookSession" : "externalSession")}
          </span>
          <h3>{session.title || t("untitledSession")}</h3>
        </div>
      </div>
      <p>
        {t(
          connected
            ? "codexHookConnected"
            : hook
              ? session.status === "exited"
                ? "codexHookDisconnected"
                : "codexHookPending"
              : "externalSessionHint",
        )}
      </p>
      {coverage && (
        <div className="codex-hook-setup">
          <p>
            {t(
              coverage.installed
                ? "codexAllWindowsInstalled"
                : "codexAllWindowsPrepared",
            )}
          </p>
          {onDisableAll && (
            <button
              className="text-button"
              disabled={busy}
              onClick={() => {
                setBusy(true);
                setError("");
                void onDisableAll()
                  .catch((error) => setError(String(error)))
                  .finally(() => setBusy(false));
              }}
            >
              {t("disableCodexAllWindows")}
            </button>
          )}
        </div>
      )}
      {session.providerId === "codex" && onConnect && (
        <button
          className="primary-button"
          disabled={busy}
          onClick={() => void connect()}
        >
          <PlugZap size={15} />
          {t(
            busy
              ? "loading"
              : coverage ||
                  ["hook-pending", "hook-connected"].includes(session.status)
                ? "showCodexHookSetup"
                : "connectCodexApprovals",
          )}
        </button>
      )}
      {setup && (
        <div className="codex-hook-setup">
          <p>{t("codexHookInstall")}</p>
          <pre>
            <code>{setup.installCommand}</code>
          </pre>
          <p>{t("codexHookTrust")}</p>
        </div>
      )}
      {error && (
        <p className="form-error" role="alert">
          {error}
        </p>
      )}
      <dl className="metadata">
        <div>
          <dt>{t("source")}</dt>
          <dd>{session.source || "Codex"}</dd>
        </div>
        <div>
          <dt>
            <FolderOpen size={12} />
            {t("project")}
          </dt>
          <dd>{session.cwd || "—"}</dd>
        </div>
        <div>
          <dt>
            <Clock3 size={12} />
            {t("updated")}
          </dt>
          <dd>
            {session.updatedAt
              ? new Date(session.updatedAt * 1000).toLocaleString(
                  locale === "zh" ? "zh-CN" : "en-US",
                )
              : "—"}
          </dd>
        </div>
        <div>
          <dt>{t("sessionIdentifier")}</dt>
          <dd>{session.id.split(":").at(-1)}</dd>
        </div>
      </dl>
    </section>
  );
}
