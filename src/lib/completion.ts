import type { TaskCompletion } from "../types";
export interface CompletionState {
  items: TaskCompletion[];
  busy: boolean;
  error: string;
}
interface CompletionCommands {
  list(): Promise<TaskCompletion[]>;
  open(sessionId: string): Promise<void>;
  dismiss(id: string): Promise<void>;
}
// The backend owns ordering and acknowledgement. A request generation prevents
// a slower pre-dismissal snapshot from putting an acknowledged card back.
export class CompletionInbox {
  state: CompletionState = { items: [], busy: false, error: "" };
  private generation = 0;
  constructor(
    private commands: CompletionCommands,
    private changed: (state: CompletionState) => void,
  ) {}
  private update(patch: Partial<CompletionState>) {
    this.state = { ...this.state, ...patch };
    this.changed(this.state);
  }
  async refresh() {
    const generation = ++this.generation;
    try {
      const items = await this.commands.list();
      if (generation === this.generation) this.update({ items, error: "" });
    } catch (error) {
      if (generation === this.generation) this.update({ error: String(error) });
    }
  }
  async act(item: TaskCompletion, action: "open" | "dismiss") {
    if (this.state.busy) return;
    this.update({ busy: true, error: "" });
    try {
      if (action === "open") await this.commands.open(item.sessionId);
      await this.commands.dismiss(item.id);
      ++this.generation;
      this.update({
        items: this.state.items.filter((entry) => entry.id !== item.id),
      });
      await this.refresh();
    } catch (error) {
      this.update({ error: String(error) });
    } finally {
      this.update({ busy: false });
    }
  }
}
