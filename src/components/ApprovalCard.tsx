import { ShieldCheck, Send } from "lucide-react";
import { useState } from "react";
import type { Interaction, Reply, Provider } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
import { approvalContent } from "../lib/approval-content";
export type ApprovalDraft = {
  answers: Record<string, string[]>;
  freeText: Record<string, string>;
};
export function ApprovalCard({
  request,
  provider,
  t,
  onRespond,
  draft,
  onDraftChange,
}: {
  request: Interaction;
  provider?: Provider;
  t: T;
  onRespond: (reply: Reply) => Promise<void>;
  expandedDetails?: boolean;
  draft?: ApprovalDraft;
  onDraftChange?: (draft: ApprovalDraft) => void;
}) {
  const [localDraft, setLocalDraft] = useState<ApprovalDraft>({
    answers: {},
    freeText: {},
  });
  const currentDraft = draft ?? localDraft;
  const { answers } = currentDraft;
  const updateDraft = (next: ApprovalDraft) => {
    setLocalDraft(next);
    onDraftChange?.(next);
  };
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function submit(reply: Reply) {
    setBusy(true);
    setError("");
    try {
      await onRespond(reply);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  const expired = request.expiresAt <= Date.now() / 1000;
  const content = approvalContent(request);
  return (
    <article className="approval-card">
      <header>
        {provider ? (
          <AgentIcon provider={provider} size={28} />
        ) : (
          <ShieldCheck size={20} />
        )}
        <div>
          <strong>{provider?.name || request.providerId}</strong>
          <span>{request.title}</span>
        </div>
        <span className="request-badge">{t("approvals")}</span>
      </header>
      <div className="approval-context">
        <strong>{request.profileLabel}</strong>
        <span title={request.cwd}>{request.cwd}</span>
        <small>{request.sessionId.slice(0, 8)}</small>
      </div>
      <div className="approval-readable-content">
        <p className="approval-reason">
          {content.reason || t("approvalReasonMissing")}
        </p>
        {content.actions.length > 0 && (
          <div
            className="approval-command-list"
            aria-label={t("approvalAction")}
          >
            {content.actions.map((action, index) => (
              <pre key={index}>
                <code>{action}</code>
              </pre>
            ))}
          </div>
        )}
      </div>
      {request.questions.map((q) => (
        <fieldset key={q.id}>
          <legend>{q.title}</legend>
          {q.options.map((o) => (
            <label className="question-option" key={o.id}>
              <input
                type={q.multiple ? "checkbox" : "radio"}
                name={`${request.id}-${q.id}`}
                checked={(answers[q.id] || []).includes(o.id)}
                onChange={() =>
                  updateDraft({
                    freeText: { ...currentDraft.freeText, [q.id]: "" },
                    answers: {
                      ...answers,
                      [q.id]: q.multiple
                        ? (answers[q.id] || []).includes(o.id)
                          ? answers[q.id].filter((x) => x !== o.id)
                          : [...(answers[q.id] || []), o.id]
                        : [o.id],
                    },
                  })
                }
              />
              {o.label}
            </label>
          ))}
          <input
            className="text-input"
            placeholder={t("other")}
            value={currentDraft.freeText[q.id] || ""}
            onChange={(e) =>
              updateDraft({
                answers: {
                  ...answers,
                  [q.id]: e.target.value ? [e.target.value] : [],
                },
                freeText: { ...currentDraft.freeText, [q.id]: e.target.value },
              })
            }
          />
        </fieldset>
      ))}
      {error && <p className="form-error">{error}</p>}
      <div className="approval-actions">
        {request.questions.length ? (
          <button
            className="primary-button"
            disabled={busy || expired}
            onClick={() => submit({ answers })}
          >
            <Send size={13} />
            {t("reply")}
          </button>
        ) : (
          request.choices.map((choice, i) => (
            <button
              key={choice.id}
              className={
                /deny|decline|cancel/i.test(choice.id + " " + choice.label)
                  ? "secondary-button"
                  : i === 0
                    ? "primary-button"
                    : "secondary-button"
              }
              disabled={busy || expired}
              onClick={() => submit({ choice: choice.id, answers: {} })}
            >
              {choice.label === "Allow once"
                ? t("allow")
                : choice.label === "Deny"
                  ? t("decline")
                  : choice.label === "Cancel"
                    ? t("cancel")
                    : choice.label === "Allow command rule"
                      ? t("allowCommandRule")
                      : choice.label === "Allow for session"
                        ? t("allowSession")
                        : choice.label}
            </button>
          ))
        )}
      </div>
    </article>
  );
}
