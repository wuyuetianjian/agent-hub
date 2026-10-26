import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { CompletionWindow } from "./components/CompletionWindow";
import { installOverlayHost } from "./lib/overlay-bridge";
import "./styles.css";

async function boot() {
  // On macOS these Tauri webviews are hidden command proxies, not another UI.
  // The Accessory helper loads this entry point without Tauri internals.
  if ("__TAURI_INTERNALS__" in window) {
    if (await invoke<boolean>("overlay_host")) {
      await installOverlayHost(window, invoke);
      return;
    }
  }
  const completionWindow = new URLSearchParams(window.location.search).has(
    "completion",
  );
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      {completionWindow ? <CompletionWindow /> : <App />}
    </React.StrictMode>,
  );
}

void boot().catch((error: unknown) => {
  // Fail closed rather than mounting duplicate panels when host setup fails.
  console.error("Orbit startup failed", error);
  const root = document.getElementById("root");
  if (root) {
    root.setAttribute("role", "alert");
    root.textContent = `Orbit could not start: ${error instanceof Error ? error.message : String(error)}`;
  }
});
