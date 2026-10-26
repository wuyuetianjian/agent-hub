import { run } from "@tauri-apps/cli";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
process.env.CARGO_HOME ??= resolve(".cache/cargo");
try {
  // The native helper deliberately cannot load Vite/network URLs. Give its
  // local asset resolver a current production bundle while workspace uses HMR.
  if (process.platform === "darwin" && process.argv[2] === "dev") {
    const build = spawnSync("npm", ["run", "build"], { stdio: "inherit" });
    if (build.error) throw build.error;
    if (build.status !== 0) throw new Error("Overlay frontend asset build failed");
  }
  await run(process.argv.slice(2), "agent-orbit");
} catch (error) {
  console.error(error.message || String(error));
  process.exitCode = 1;
}
