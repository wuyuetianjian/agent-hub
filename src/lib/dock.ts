import type { AgentView, Interaction, SessionInfo } from "../types.ts";
export function ringOffset(
  percent: number | null,
  radius: number,
): number | null {
  return percent == null ||
    !Number.isFinite(percent) ||
    percent < 0 ||
    percent > 100
    ? null
    : 2 * Math.PI * radius * (1 - percent / 100);
}
export function requestsForProfile(
  profileId: string,
  sessions: SessionInfo[],
  requests: Interaction[],
): Interaction[] {
  const ids = new Set(
    sessions
      .filter((s) => !s.readOnly && s.profileId === profileId)
      .map((s) => s.id),
  );
  return requests.filter((r) => ids.has(r.sessionId));
}
export function dockAgents(
  agents: AgentView[],
  visible: string[],
  sessions: SessionInfo[],
  requests: Interaction[],
): AgentView[] {
  const pending = new Set(
    sessions
      .filter((s) => !s.readOnly && requests.some((r) => r.sessionId === s.id))
      .map((s) => s.profileId),
  );
  return agents
    .filter(
      (a) =>
        (a.profile.enabled !== false && visible.includes(a.provider.id)) ||
        pending.has(a.profile.id),
    )
    .sort((a, b) => {
      const ai = visible.indexOf(a.provider.id),
        bi = visible.indexOf(b.provider.id);
      return (ai < 0 ? Infinity : ai) - (bi < 0 ? Infinity : bi);
    });
}

export function profileActivity(
  profileId: string,
  sessions: SessionInfo[],
  requests: Interaction[],
): "idle" | "running" | "waiting" {
  if (requestsForProfile(profileId, sessions, requests).length)
    return "waiting";
  return sessions.some(
    (session) =>
      session.profileId === profileId &&
      !session.readOnly &&
      session.status === "running",
  )
    ? "running"
    : "idle";
}
