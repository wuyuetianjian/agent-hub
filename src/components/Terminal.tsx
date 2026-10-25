import { useEffect, useRef, useState } from "react";
import { Terminal as XTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { call, onEvent } from "../api";
import type { SessionInfo } from "../types";
import type { T } from "../i18n";
import { Send } from "lucide-react";
function bytes(encoded: string) {
  return Uint8Array.from(atob(encoded), (c) => c.charCodeAt(0));
}
export function Terminal({
  session,
  t,
  onError,
}: {
  session: SessionInfo;
  t: T;
  onError: (message: string) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [prompt, setPrompt] = useState("");
  const [sending, setSending] = useState(false);
  const errorRef = useRef(onError);
  errorRef.current = onError;
  useEffect(() => {
    if (!host.current) return;
    const terminal = new XTerminal({
      fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", monospace',
      fontSize: 12,
      lineHeight: 1.45,
      convertEol: true,
      cursorBlink: true,
      theme: {
        background: "#151816",
        foreground: "#e1e6dd",
        cursor: "#b4d6ba",
      },
      allowProposedApi: false,
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host.current);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let sequence = -1;
    let ready = false;
    const waiting: { bytes: string; sequence: number }[] = [];
    const write = (data: { bytes: string; sequence: number }) => {
      if (data.sequence > sequence) {
        terminal.write(bytes(data.bytes));
        sequence = data.sequence;
      }
    };
    const resize = () => {
      if (disposed) return;
      try {
        fit.fit();
        void call("resize_session", {
          sessionId: session.id,
          cols: terminal.cols,
          rows: terminal.rows,
        }).catch(() => {});
      } catch {
        /* hidden terminal will resize when visible */
      }
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host.current);
    const input = terminal.onData((data) => {
      if (session.mode === "terminal")
        void call("write_session", { sessionId: session.id, data }).catch((e) =>
          errorRef.current(String(e)),
        );
    });
    void (async () => {
      unlisten = await onEvent((e) => {
        if (
          e.kind === "output" &&
          e.sessionId === session.id &&
          e.data.bytes &&
          e.data.sequence != null
        ) {
          const v = e.data as { bytes: string; sequence: number };
          if (ready) write(v);
          else waiting.push(v);
        }
      });
      if (disposed) {
        unlisten();
        return;
      }
      try {
        const replay = await call<{ bytes: string; sequence: number }>(
          "session_replay",
          { sessionId: session.id },
        );
        if (disposed) return;
        write(replay);
        ready = true;
        waiting.forEach(write);
        resize();
      } catch (e) {
        if (!disposed) errorRef.current(String(e));
      }
    })();
    return () => {
      disposed = true;
      unlisten?.();
      observer.disconnect();
      input.dispose();
      terminal.dispose();
    };
  }, [session.id, session.mode]);
  async function send() {
    if (!prompt.trim()) return;
    setSending(true);
    try {
      await call("prompt_session", { sessionId: session.id, text: prompt });
      setPrompt("");
    } catch (e) {
      onError(String(e));
    } finally {
      setSending(false);
    }
  }
  return (
    <div className="terminal-layout">
      <div className="terminal-host" ref={host} />
      {session.mode === "structured" && (
        <form
          className="prompt-form"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <input
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder={t("prompt")}
            disabled={session.status !== "running"}
          />
          <button
            className="icon-button"
            title={t("send")}
            disabled={sending || session.status !== "running"}
          >
            <Send size={16} />
          </button>
        </form>
      )}
    </div>
  );
}
