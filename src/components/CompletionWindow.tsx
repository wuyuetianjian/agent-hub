import { useEffect, useState } from "react";
import { CheckCircle2, ArrowUpRight, X } from "lucide-react";
import type { Dashboard, TaskCompletion } from "../types";
import { translator, type T } from "../i18n";
import { call, onEvent } from "../api";
import { CompletionInbox, type CompletionState } from "../lib/completion";

export function CompletionCard({
  completion,
  count,
  busy,
  error,
  t,
  onOpen,
  onDismiss,
}: {
  completion: TaskCompletion;
  count: number;
  busy: boolean;
  error: string;
  t: T;
  onOpen: () => void;
  onDismiss: () => void;
}) {
  return (
    <section className="completion-card" aria-label={t("taskCompleted")}>
      <header>
        <CheckCircle2 size={19} aria-hidden="true" />
        <h2 role="status">{t("taskCompleted")}</h2>
        <button
          className="completion-close"
          disabled={busy}
          aria-label={t("dismissCompletion")}
          title={t("dismissCompletion")}
          onClick={onDismiss}
        >
          <X size={17} />
        </button>
      </header>
      <strong className="completion-account" title={completion.profileLabel}>
        {completion.profileLabel}
      </strong>
      <p className="completion-project" title={completion.cwd}>
        {completion.cwd}
      </p>
      <p className="completion-session" title={completion.sessionId}>
        {t("sessionIdentifier")}: {completion.sessionId.slice(0, 12)}
      </p>
      {error && (
        <p className="completion-error" role="alert">
          {error}
        </p>
      )}
      <footer>
        <span>{count > 1 ? `${count} ${t("completionQueue")}` : ""}</span>
        <button className="completion-open" disabled={busy} onClick={onOpen}>
          {t("viewCompletedSession")}
          <ArrowUpRight size={15} aria-hidden="true" />
        </button>
      </footer>
    </section>
  );
}

export function CompletionWindow() {
  const [state, setState] = useState<CompletionState>({
    items: [],
    busy: false,
    error: "",
  });
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [inbox, setInbox] = useState<CompletionInbox | null>(null);
  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    let settingsGeneration = 0;
    const controller = new CompletionInbox(
      {
        list: () => call<TaskCompletion[]>("get_task_completions"),
        open: (sessionId) =>
          call<void>("open_workspace", { page: "sessions", sessionId }),
        dismiss: (id) => call<void>("dismiss_task_completion", { id }),
      },
      (next) => {
        if (!disposed) setState(next);
      },
    );
    setInbox(controller);
    const refresh = async () => {
      const generation = ++settingsGeneration;
      void controller.refresh();
      try {
        const next = await call<Dashboard>("get_dashboard");
        if (!disposed && generation === settingsGeneration) setDashboard(next);
      } catch (error) {
        if (!disposed)
          setState((current) => ({ ...current, error: String(error) }));
      }
    };
    // Subscribe before the initial query so a completion during webview startup
    // cannot be lost. The query also reconciles native visibility after reload.
    void onEvent((event) => {
      if (event.kind === "dashboard" || event.kind === "task-completed")
        void refresh();
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else {
          off = unlisten;
          void refresh();
        }
      })
      .catch((error) => {
        if (!disposed)
          setState((current) => ({ ...current, error: String(error) }));
      });
    const timer = setInterval(() => {
      if (!disposed) void refresh();
    }, 15000);
    return () => {
      disposed = true;
      off?.();
      clearInterval(timer);
    };
  }, []);
  const t = translator(dashboard?.settings.locale ?? "zh");
  const configuredTheme = dashboard?.settings.theme ?? "system";
  const theme =
    configuredTheme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : configuredTheme;
  const item = state.items[0];
  return (
    <main className="app-frame completion-frame" data-theme={theme}>
      {item ? (
        <CompletionCard
          completion={item}
          count={state.items.length}
          busy={state.busy}
          error={state.error}
          t={t}
          onOpen={() => {
            void inbox?.act(item, "open");
          }}
          onDismiss={() => {
            void inbox?.act(item, "dismiss");
          }}
        />
      ) : (
        <p className="completion-empty" role="status">
          {state.error || t("noCompletions")}
        </p>
      )}
    </main>
  );
}
