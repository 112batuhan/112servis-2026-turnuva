import { Component, type ErrorInfo, type ReactNode } from "react";
import "./ErrorBoundary.css";

interface Props {
  children: ReactNode;
  /**
   * What shows instead of what failed: "page" a message with a reload button, "none"
   * nothing at all (the stream overlay, so the broadcast stays clean).
   */
  fallback?: "page" | "none";
  /** Tries drawing it again this long after it failed, for screens nobody can click (OBS). */
  retryAfterMs?: number;
}

interface State {
  failed: boolean;
}

/**
 * Keeps one exception during render from blanking the whole app. Key it by the path so
 * navigating away starts over.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false };
  private retry: ReturnType<typeof setTimeout> | null = null;

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("render failed", error, info.componentStack);
    if (this.props.retryAfterMs !== undefined) {
      this.retry = setTimeout(() => this.setState({ failed: false }), this.props.retryAfterMs);
    }
  }

  componentWillUnmount() {
    if (this.retry) clearTimeout(this.retry);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    if (this.props.fallback === "none") return null;
    return (
      <div className="error-boundary" role="alert">
        <p className="error-boundary-title">Bir şeyler ters gitti</p>
        <p className="error-boundary-text">Bu sayfa gösterilirken bir hata oluştu.</p>
        <button
          type="button"
          className="btn btn-secondary"
          onClick={() => window.location.reload()}
        >
          Yeniden yükle
        </button>
      </div>
    );
  }
}
