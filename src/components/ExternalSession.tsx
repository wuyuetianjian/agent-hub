import { History, FolderOpen, Clock3 } from "lucide-react";
import type { SessionInfo } from "../types";
import type { T } from "../i18n";
export function ExternalSession({
  session,
  t,
  locale,
}: {
  session: SessionInfo;
  t: T;
  locale: string;
}) {
  return (
    <section className="external-session" aria-label={t("externalSession")}>
      <div className="external-session-heading">
        <History size={23} />
        <div>
          <span className="status-chip">{t("externalSession")}</span>
          <h3>{session.title || t("untitledSession")}</h3>
        </div>
      </div>
      <p>{t("externalSessionHint")}</p>
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
