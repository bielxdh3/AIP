import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Bubble from "./Bubble";
import Overlay from "./Overlay";
import { ThemeProvider } from "./theme";

class RenderErrorBoundary extends React.Component<
  React.PropsWithChildren,
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="conversation-empty" role="alert">
          <p>A interface encontrou um erro.</p>
          <small>Código: frontend_render_failed</small>
          <button type="button" onClick={() => window.location.reload()}>
            Recarregar interface
          </button>
        </main>
      );
    }
    return this.props.children;
  }
}

const parameters = new URLSearchParams(window.location.search);
const agentId = parameters.get("agent");
const bubbleAgentId = parameters.get("bubble");
document.documentElement.dataset.surface = agentId
  ? "overlay"
  : bubbleAgentId
    ? "bubble"
    : "panel";
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <RenderErrorBoundary>
    <React.StrictMode>
      <ThemeProvider>
        {agentId ? (
          <Overlay agentId={agentId} />
        ) : bubbleAgentId ? (
          <Bubble agentId={bubbleAgentId} />
        ) : (
          <App />
        )}
      </ThemeProvider>
    </React.StrictMode>
  </RenderErrorBoundary>,
);
