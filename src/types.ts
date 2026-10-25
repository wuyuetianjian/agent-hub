export interface Provider {
  id: string;
  name: string;
  publisher: string;
  icon: string;
  color: string;
  executables: string[];
  adapter: string;
  usageUrl: string;
  credentialPaths: string[];
  detectionPaths: string[];
}
export interface Settings {
  locale: "zh" | "en";
  theme: "dark" | "light" | "system";
  refreshSeconds: number;
  staleSeconds: number;
  requestTimeoutSeconds: number;
  interactionTimeoutSeconds: number;
  warningPercent: number;
  alwaysOnTop: boolean;
  compact: boolean;
  startAtLogin: boolean;
  visibleProviders: string[];
  windowWidth: number;
  windowHeight: number;
  expandedWidth: number;
  expandedHeight: number;
  maxOutputBytes: number;
  windowMode: "floating" | "island";
  summaryPolicy: "shortest" | "lowest";
  islandWidth: number;
  islandHeight: number;
}
export interface Profile {
  id: string;
  providerId: string;
  label: string;
  executable: string;
  args: string[];
  configDir: string;
  quotaCommand: string;
  quotaArgs: string[];
  quotaFile: string;
  enabled: boolean;
}
export interface QuotaWindow {
  label: string;
  remainingPercent: number | null;
  used: number | null;
  limit: number | null;
  unit: string;
  resetsAt: number | null;
  windowMinutes?: number | null;
}
export interface Snapshot {
  profileId: string;
  account: string | null;
  plan: string | null;
  status: string;
  source: string;
  message: string;
  windows: QuotaWindow[];
  fetchedAt: number;
}
export interface AgentView {
  provider: Provider;
  profile: Profile;
  discovery: {
    installed: boolean;
    executable: string | null;
    configFound: boolean;
  };
  snapshot: Snapshot;
}
export interface SessionInfo {
  id: string;
  profileId: string;
  providerId: string;
  cwd: string;
  mode: string;
  status: string;
  startedAt: number;
}
export interface Question {
  id: string;
  title: string;
  options: { id: string; label: string }[];
  multiple: boolean;
}
export interface Interaction {
  profileLabel: string;
  cwd: string;
  id: string;
  sessionId: string;
  providerId: string;
  title: string;
  detail: string;
  kind: string;
  choices: { id: string; label: string }[];
  questions: Question[];
  createdAt: number;
  expiresAt: number;
}
export interface Reply {
  choice?: string;
  answers: Record<string, string[]>;
}
export interface Dashboard {
  agents: AgentView[];
  settings: Settings;
  sessions: SessionInfo[];
  interactions: Interaction[];
  platform: string;
}
export interface AppEvent {
  kind: string;
  sessionId: string | null;
  data: { bytes?: string; sequence?: number };
}
