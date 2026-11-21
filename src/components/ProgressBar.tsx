import { progressPercent, progressText } from "../format";
import type { DownloadStatus } from "../types";
import "./Task.css";

export function ProgressBar({ received, total, status, label, showText = true }: { received: number; total: number | null; status: DownloadStatus; label: string; showText?: boolean }) {
  const percent = progressPercent(received, total, status);
  const text = progressText({ bytes_received: received, total_bytes: total, status });
  return <div className="progress-wrap">
    <div className={`progress-track progress-${status} ${percent === null ? "progress-unknown" : ""}`} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent === null ? undefined : Math.round(percent)} aria-valuetext={text}>
      <div className="progress-fill" style={percent === null ? undefined : { width: `${percent}%` }} />
    </div>
    {showText && <span className="progress-text">{text}</span>}
  </div>;
}
