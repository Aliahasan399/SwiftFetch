import { Component, type ErrorInfo, type ReactNode } from "react";

/**
 * Keep a frontend runtime exception from leaving an empty, unrecoverable
 * WebView window. The user gets a visible explanation and a safe reload.
 */
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("SwiftFetch UI error", error.message, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="container" style={{ padding: 32 }}>
        <h1>SwiftFetch needs to reload</h1>
        <p>The interface hit an unexpected error. Your downloads, settings and history are stored locally and are not deleted by reloading.</p>
        <pre className="diagnostic-json" tabIndex={0}>{this.state.error.message}</pre>
        <button type="button" className="button button-primary" onClick={() => window.location.reload()}>Reload SwiftFetch</button>
      </main>
    );
  }
}
