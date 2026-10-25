import { run } from "@tauri-apps/cli";
import { resolve } from "node:path";
process.env.CARGO_HOME ??= resolve(".cache/cargo");
try {
  await run(process.argv.slice(2), "agent-orbit");
} catch (error) {
  console.error(error.message || String(error));
  process.exitCode = 1;
}
