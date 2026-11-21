import type { ReactNode } from "react";
import { errorGuidance } from "../format";
import type { IpcError } from "../ipc";
import { Icon } from "./Icon";

export function ErrorBanner({ error, onRetry }: { error: IpcError | null; onRetry?: () => void }) {
  if (!error) return null;
  const guidance = errorGuidance(error.code);
  return <div className="notice notice-error" role="alert">
    <Icon name="warning" />
    <div><strong>{error.message}</strong>{guidance && <p>{guidance}</p>}<span className="error-code">{error.code}</span></div>
    {onRetry && <button type="button" className="button button-small" onClick={onRetry}>Try again</button>}
  </div>;
}

export function InfoBanner({ children, warning = false }: { children: ReactNode; warning?: boolean }) {
  return <div className={`notice ${warning ? "notice-warning" : "notice-info"}`}><Icon name={warning ? "warning" : "info"} /><div>{children}</div></div>;
}

export function LoadingState({ label = "Loading from the desktop core…" }: { label?: string }) {
  return <div className="loading-state" role="status"><span className="spinner" aria-hidden="true" />{label}</div>;
}
