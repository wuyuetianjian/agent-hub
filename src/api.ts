import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import defaults from "../config/defaults.json";
import type { AppEvent, Dashboard, Settings, Profile } from "./types";
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let preview: Dashboard = {
  settings: defaults.settings as Settings,
  platform:
    typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")
      ? "macos"
      : "linux",
  sessions: [],
  interactions: [],
  agents: defaults.providers.map((provider) => ({
    provider,
    profile: {
      id: provider.id,
      providerId: provider.id,
      label: provider.name,
      executable: "",
      args: [],
      configDir: "",
      quotaCommand: "",
      quotaArgs: [],
      quotaFile: "",
      enabled: true,
    },
    discovery: { installed: false, executable: null, configFound: false },
    snapshot: {
      profileId: provider.id,
      account: null,
      plan: null,
      status: "unavailable",
      source: "",
      message: "Desktop connection required",
      windows: [],
      fetchedAt: 0,
    },
  })),
};
export async function call<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (isDesktop) return invoke<T>(command, args);
  if (command === "get_dashboard") return structuredClone(preview) as T;
  if (command === "save_settings") {
    preview = { ...preview, settings: args.settings as Settings };
    return undefined as T;
  }
  if (command === "save_profile") {
    const profile = args.profile as Profile;
    const existing = preview.agents.find((a) => a.profile.id === profile.id);
    if (existing) existing.profile = profile;
    return undefined as T;
  }
  if (command === "get_usage") {
    const tokens = {
      total: 0,
      input: 0,
      output: 0,
      cacheRead: 0,
      cacheWrite: 0,
      reasoning: 0,
    };
    return {
      total: tokens,
      buckets: (args.intervals as { start: number; end: number }[]).map(
        (interval) => ({ ...interval, tokens }),
      ),
      agents: [],
      sources: preview.agents.map((agent) => ({
        profileId: agent.profile.id,
        providerId: agent.provider.id,
        status: "noLogs",
        files: 0,
        warnings: 0,
        hasData: false,
      })),
      scannedAt: Date.now() / 1000,
      partial: false,
    } as T;
  }
  if (command === "set_window_view") return undefined as T;
  throw new Error(
    "此操作需要桌面应用 / Launch the desktop app for this action.",
  );
}
export async function onEvent(
  callback: (event: AppEvent) => void,
): Promise<() => void> {
  if (!isDesktop) return () => {};
  return listen<AppEvent>("orbit:event", (event) => callback(event.payload));
}
export async function chooseDirectory(): Promise<string | null> {
  if (!isDesktop) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}
export async function openUrl(url: string) {
  if (!/^https?:\/\//.test(url))
    throw new Error("Only HTTP(S) URLs are supported");
  if (isDesktop) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
