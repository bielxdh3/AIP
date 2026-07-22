import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Bubble from "./Bubble";
import Overlay from "./Overlay";

const parameters = new URLSearchParams(window.location.search);
const agentId = parameters.get("agent");
const bubbleAgentId = parameters.get("bubble");
document.documentElement.dataset.surface = agentId
  ? "overlay"
  : bubbleAgentId
    ? "bubble"
    : "panel";
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {agentId ? (
      <Overlay agentId={agentId} />
    ) : bubbleAgentId ? (
      <Bubble agentId={bubbleAgentId} />
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
