import type { Interaction } from "../types";
export type ApprovalContent = { reason: string | null; actions: string[] };
const text = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;
export function approvalContent(
  request: Pick<Interaction, "detail">,
): ApprovalContent {
  let payload: Record<string, unknown>;
  try {
    const parsed: unknown = JSON.parse(request.detail);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
      return { reason: null, actions: [] };
    payload = parsed as Record<string, unknown>;
  } catch {
    return { reason: null, actions: [] };
  }
  const reason = text(payload.reason);
  const actions: string[] = [];
  if (Array.isArray(payload.commandActions)) {
    for (const action of payload.commandActions) {
      if (action && typeof action === "object") {
        const command = text((action as Record<string, unknown>).command);
        if (command) actions.push(command);
      }
    }
  }
  const command = text(payload.command);
  if (!actions.length && command) actions.push(command);
  const changes = payload.changes;
  if (changes && typeof changes === "object" && !Array.isArray(changes))
    actions.push(...Object.keys(changes));
  // Legacy Claude bridge records tool_input directly.
  for (const key of ["file_path", "path"]) {
    const path = text(payload[key]);
    if (path) actions.push(path);
  }
  return { reason, actions: [...new Set(actions)] };
}
