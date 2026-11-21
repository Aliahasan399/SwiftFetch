import type { DownloadStatus, TaskSummary } from "./types";

const decimal = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 });
const integer = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
const dateTime = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
const byteUnits = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

export function formatNumber(value: number): string {
  return integer.format(Number.isFinite(value) ? value : 0);
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return "Unknown size";
  if (bytes === 0) return "0 B";
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), byteUnits.length - 1);
  const unit = Math.max(0, exponent);
  return `${decimal.format(bytes / 1024 ** unit)} ${byteUnits[unit]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(Math.max(0, bytesPerSecond))}/s`;
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "Estimating…";
  if (seconds < 1) return "Less than a second";
  const rounded = Math.ceil(seconds);
  if (rounded < 60) return `${formatNumber(rounded)}s`;
  if (rounded < 3600) return `${formatNumber(Math.floor(rounded / 60))}m ${rounded % 60}s`;
  if (rounded < 86400) return `${formatNumber(Math.floor(rounded / 3600))}h ${Math.floor((rounded % 3600) / 60)}m`;
  return `${formatNumber(Math.floor(rounded / 86400))}d ${Math.floor((rounded % 86400) / 3600)}h`;
}

export function formatDate(value: string | number | null | undefined): string {
  if (value == null) return "Not reported";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Unknown date" : dateTime.format(date);
}

export const statusLabels: Record<DownloadStatus, string> = {
  queued: "Queued", starting: "Starting", downloading: "Downloading", paused: "Paused",
  retry_wait: "Waiting to retry", verifying: "Verifying", scanning: "Scanning",
  completed: "Completed", failed: "Failed", cancelled: "Cancelled",
};

export function isTerminal(status: DownloadStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

export function isActive(status: DownloadStatus): boolean {
  return status === "starting" || status === "downloading" || status === "verifying" || status === "scanning";
}

export function progressPercent(received: number, total: number | null, status: DownloadStatus): number | null {
  if (status === "completed") return 100;
  if (total == null || total <= 0 || !Number.isFinite(total)) return null;
  return Math.min(100, Math.max(0, (received / total) * 100));
}

export function progressText(task: Pick<TaskSummary, "bytes_received" | "total_bytes" | "status">): string {
  const percent = progressPercent(task.bytes_received, task.total_bytes, task.status);
  const amount = `${formatBytes(task.bytes_received)} of ${formatBytes(task.total_bytes)}`;
  if (task.status === "completed") return `Completed · ${amount}`;
  if (percent === null) return `${amount} · ${statusLabels[task.status]}`;
  const waiting = percent >= 100 ? "; awaiting core finalization" : "";
  return `${decimal.format(percent)}% transferred · ${amount} · ${statusLabels[task.status]}${waiting}`;
}

export function safeUrl(value: string | null): string {
  if (!value) return "Not reported";
  try {
    const url = new URL(value);
    if (url.protocol !== "http:" && url.protocol !== "https:") return "[unsupported URL hidden]";
    return `${url.protocol}//${url.host}${url.pathname}${url.search ? "?[query hidden]" : ""}`;
  } catch {
    return "[invalid URL hidden]";
  }
}

// Render only safe error text. Never forward a raw IPC rejection to the UI/logs.
export function redactSensitiveText(value: string): string {
  return value
    .replace(/https?:\/\/[^\s<>"']+/gi, "[URL hidden]")
    .replace(/(?:authorization|proxy-authorization|cookie|set-cookie)\s*[:=][^\r\n]*/gi, "[sensitive header hidden]")
    .replace(/\b(?:token|api[_-]?key|password|secret)\s*[:=]\s*[^\s,;]+/gi, "[credential hidden]")
    .replace(/\b[A-Za-z]:[\\/][^\r\n]*/g, "[local path hidden]");
}

export function validateDownloadUrl(raw: string): string | null {
  const value = raw.trim();
  if (!value) return "Enter a download URL.";
  if (new TextEncoder().encode(value).length > 2048) return "The URL must be at most 2,048 UTF-8 bytes.";
  if (/[\u0000-\u001f\u007f]/.test(value)) return "URLs cannot contain control characters.";
  try {
    const url = new URL(value);
    if (url.protocol !== "http:" && url.protocol !== "https:") return "Only HTTP and HTTPS downloads are supported.";
    if (!url.hostname) return "Enter a URL with a valid host.";
    if (url.username || url.password) return "Credentials in URLs are not allowed. Use a permitted link without embedded credentials.";
    return null;
  } catch {
    return "Enter a complete URL, starting with https:// or http://.";
  }
}

export function validateFilename(value: string): string | null {
  if (!value) return null;
  if (new TextEncoder().encode(value).length > 200) return "The filename must be at most 200 UTF-8 bytes.";
  if (/[<>:"/\\|?*\u0000-\u001f]/.test(value) || /[. ]$/.test(value)) return "Use a filename without path separators, reserved characters, or trailing dots/spaces.";
  if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(value) || value === "." || value === "..") return "That filename is reserved by Windows.";
  return null;
}

export function errorGuidance(code: string | null | undefined): string | null {
  switch (code) {
    case "CORE_UNAVAILABLE": return "Open the SwiftFetch desktop app to connect to the Rust core. A browser preview cannot download files.";
    case "COMMAND_UNAVAILABLE": return "This command is not available in the connected core. Keep your input and update or reconnect the desktop backend.";
    case "NETWORK_ERROR": case "OFFLINE": return "The network may be offline. Your queue is preserved; reconnect, then resume or retry.";
    case "DISK_FULL": return "Free space on the destination drive, then retry. Open task details for the path and remaining byte estimate. Free-space telemetry is not exposed by the core.";
    case "SERVER_DOES_NOT_SUPPORT_RANGES": return "This server does not support segmented transfers. The core uses a single stream when possible; additional connections cannot accelerate this server.";
    case "RESUME_INCOMPATIBLE": case "RESUME_METADATA_INVALID": return "The server's copy or the saved resume metadata changed. Resuming may restart from zero; confirm before discarding partial progress.";
    case "ANTIVIRUS_UNAVAILABLE": case "SCANNER_UNAVAILABLE": case "SCAN_FAILED": case "SCANNER_FAILED": return "Antivirus scanning is incomplete. Check the scanner in Settings. This file has not been confirmed safe; no automatic execution will occur.";
    case "BROWSER_BRIDGE_UNAVAILABLE": return "Paste the permitted download URL into Add download. The form keeps your URL when a request fails; browser handoff is not required.";
    case "CHECKSUM_MISMATCH": return "The file did not match the expected checksum. Do not open it; confirm the trusted checksum and source before retrying.";
    case "DESTINATION_EXISTS": case "OVERWRITE_CONFIRMATION_REQUIRED": return "The destination file already exists. Open task details to choose a new name, replace it, or skip this file.";
    case "PERMISSION_DENIED": case "INVALID_DESTINATION": return "Check that the destination exists and is writable. Choose an allowed folder for a new download.";
    case "RATE_LIMITED": return "The server has requested a delay. The core will honor its retry policy; avoid creating duplicate downloads.";
    default: return null;
  }
}

export const weekDays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export function formatDays(mask: number): string {
  if (mask === 127) return "Every day";
  return weekDays.filter((_, index) => (mask & (1 << index)) !== 0).join(", ") || "No days selected";
}
