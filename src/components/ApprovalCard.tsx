import { ShieldCheck, ChevronDown, Send } from "lucide-react";
import { useState } from "react";
import type { Interaction, Reply, Provider } from "../types";
import type { T } from "../i18n";
import { AgentIcon } from "./AgentIcon";
export function ApprovalCard({
  request,
  provider,
  t,
  onRespond,
}: {
  request: Interaction;
  provider?: Provider;
  t: T;
  onRespond: (reply: Reply) => Promise<void>;
}) {
  const [answers, setAnswers] = useState<Record<string, string[]>>({});
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
      <details>
        <summary>
          {t("details")}
          <ChevronDown size={13} />
        </summary>
        <pre>{request.detail}</pre>
      </details>
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
                  setAnswers((current) => ({
                    ...current,
                    [q.id]: q.multiple
                      ? (current[q.id] || []).includes(o.id)
                        ? current[q.id].filter((x) => x !== o.id)
                        : [...(current[q.id] || []), o.id]
                      : [o.id],
                  }))
                }
              />
              {o.label}
            </label>
          ))}
          <input
            className="text-input"
            placeholder={t("other")}
            onChange={(e) =>
              setAnswers((a) => ({ ...a, [q.id]: [e.target.value] }))
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
                  : choice.label}
            </button>
          ))
        )}
      </div>
    </article>
  );
}
