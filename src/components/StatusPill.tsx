import { statusLabels } from "../format";
import type { DownloadStatus } from "../types";
import "./Task.css";

export function StatusPill({ status }: { status: DownloadStatus }) {
  return <span className={`status-pill status-${status}`}><span className="status-dot" aria-hidden="true" />{statusLabels[status]}</span>;
}
