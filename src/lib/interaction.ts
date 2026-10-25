import type { Provider, SessionInfo } from "../types";
export function defaultSessionMode(
  provider?: Pick<Provider, "id" | "defaultSessionMode">,
): "structured" | "terminal" {
  return provider?.id === "codex" && provider.defaultSessionMode !== "terminal"
    ? "structured"
    : "terminal";
}
export function approvalConnection(
  profileId: string,
  sessions: SessionInfo[],
): "connected" | "pending" | "terminal" | "history" | "none" {
  const own = sessions.filter((session) => session.profileId === profileId);
  if (
    own.some(
      (s) =>
        s.interactionChannel === "codex-hook" && s.status === "hook-connected",
    )
  )
    return "connected";
  const live = own.filter(
    (session) => !session.readOnly && session.status === "running",
  );
  if (
    live.some(
      (session) =>
        session.interactionChannel === "codex-rpc" ||
        session.interactionChannel === "claude-hook",
    )
  )
    return "connected";
  if (
    own.some(
      (s) =>
        s.interactionChannel === "codex-hook" && s.status === "hook-pending",
    )
  )
    return "pending";
  if (live.length) return "terminal";
  return own.some((session) => session.readOnly) ? "history" : "none";
}
