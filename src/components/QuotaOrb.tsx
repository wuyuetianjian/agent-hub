import { translator } from "../i18n";
import type { AgentView, Settings } from "../types";
import { AgentIcon } from "./AgentIcon";
import { summaryWindow } from "../lib/summary";
import { ringOffset } from "../lib/dock";
export function QuotaOrb({
  agent,
  settings,
  pending = 0,
  active,
  activity = "idle",
  magnified = false,
  onHover,
  onLeave,
  onSelect,
}: {
  agent: AgentView;
  settings: Settings;
  pending?: number;
  active: boolean;
  activity?: "idle" | "running" | "waiting";
  magnified?: boolean;
  onHover: () => void;
  onLeave: () => void;
  onSelect: () => void;
}) {
  const window = summaryWindow(agent.snapshot.windows, settings.summaryPolicy);
  const percent = window?.remainingPercent ?? null;
  const offset = ringOffset(percent, 28);
  const known = offset != null;
  const low = known && percent! <= settings.warningPercent;
  const label = `${agent.provider.name} · ${agent.profile.label} · ${window?.label || "quota"} · ${known ? `${Math.round(percent!)}%` : "—"}`;
  return (
    <button
      className={`dock-orb activity-${activity} ${magnified ? "magnified" : ""} ${active ? "active" : ""} ${pending ? "has-request" : ""} ${agent.snapshot.status === "stale" ? "stale" : ""}`}
      style={
        {
          "--orb-scale": magnified
            ? settings.dockHoverSize / settings.dockSize
            : 1,
          "--orb-rest-size": `${settings.dockSize}px`,
          "--orb-color": low ? "var(--warning)" : agent.provider.color,
        } as React.CSSProperties
      }
      aria-label={label}
      aria-expanded={active}
      aria-haspopup="dialog"
      onMouseEnter={onHover}
      onMouseLeave={onLeave}
      onFocus={onHover}
      onClick={onSelect}
    >
      <span className="dock-orb-core">
        <AgentIcon provider={agent.provider} size={settings.dockSize * 0.72} />
      </span>
      <svg
        className={`orb-ring ${known ? "" : "unknown"}`}
        viewBox="0 0 64 64"
        role="progressbar"
        aria-label={`${agent.provider.name} remaining quota`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={known ? percent! : undefined}
        aria-valuetext={known ? `${Math.round(percent!)}%` : "Unknown"}
      >
        <circle className="orb-ring-track" cx="32" cy="32" r="28" />
        <circle
          className="orb-ring-value"
          cx="32"
          cy="32"
          r="28"
          strokeDasharray={known ? 2 * Math.PI * 28 : "3 7"}
          strokeDashoffset={offset ?? 0}
        />
      </svg>
      {agent.snapshot.windows.some(
        (other) => other !== window && other.remainingPercent === 0,
      ) && (
        <span
          className="orb-quota-alert"
          role="img"
          aria-label={translator(settings.locale)("otherQuotaExhausted")}
        />
      )}
      {activity === "running" && pending === 0 && (
        <span
          className="orb-live-dot"
          role="img"
          aria-label={translator(settings.locale)("hudRunning")}
        />
      )}
      {pending > 0 && (
        <span className="orb-request-count" aria-label={`${pending} pending`}>
          {pending}
        </span>
      )}
    </button>
  );
}
