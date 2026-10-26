import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CompletionWindow } from "./components/CompletionWindow";
const completionWindow = new URLSearchParams(window.location.search).has(
  "completion",
);
import "./styles.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {completionWindow ? <CompletionWindow /> : <App />}
  </React.StrictMode>,
);
