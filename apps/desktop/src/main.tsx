import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Overlay from "./Overlay";

const agentId = new URLSearchParams(window.location.search).get("agent");
document.documentElement.dataset.surface = agentId ? "overlay" : "panel";
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {agentId ? <Overlay agentId={agentId} /> : <App />}
  </React.StrictMode>,
);
