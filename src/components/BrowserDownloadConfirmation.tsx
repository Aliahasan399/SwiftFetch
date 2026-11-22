import { useEffect, useState } from "react";
import { formatBytes } from "../format";
import { api, asIpcError, type IpcError } from "../ipc";
import type { CategoryDto, MediaVariantDto, PendingConfirmation, QueueDto } from "../types";
import { ErrorBanner, InfoBanner } from "./Feedback";
import { Icon } from "./Icon";
import { Dialog } from "./Dialog";
import "./BrowserDownloadConfirmation.css";

/**
 * Original SwiftFetch confirmation for a browser-originated download.
 * No task is created until the user explicitly chooses Start or Add to queue.
 */
export function BrowserDownloadConfirmation({
  pending,
  categories,
  queues,
  onClose,
  onResolved,
}: {
  pending: PendingConfirmation;
  categories: CategoryDto[];
  queues: QueueDto[];
  onClose: () => void;
  onResolved: () => void;
}) {
  const [name, setName] = useState(pending.suggested_name || "");
  const [destination, setDestination] = useState("");
  const [category, setCategory] = useState("");
  const [queue, setQueue] = useState("");
  const [size, setSize] = useState(pending.size_hint);
  const [contentType, setContentType] = useState(pending.mime || pending.content_type);
  const [probing, setProbing] = useState(true);
  const [variants, setVariants] = useState<MediaVariantDto[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const [manifestMessage, setManifestMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    let disposed = false;
    setProbing(true);
    api.probeDownloadMetadata(pending.url)
      .then((probe) => {
        if (disposed) return;
        if (probe.size !== null) setSize(probe.size);
        if (probe.content_type) setContentType(probe.content_type);
        if (!name && probe.suggested_name) setName(probe.suggested_name);
      })
      .catch(() => {
        // A failed probe is not a failed download: keep the browser hints.
      })
      .finally(() => { if (!disposed) setProbing(false); });
    return () => { disposed = true; };
  }, [pending.url, pending.id]);

  useEffect(() => {
    const lower = pending.url.toLowerCase();
    if (pending.kind !== "media" || (!lower.includes(".m3u8") && !lower.includes(".mpd"))) return;
    let disposed = false;
    api.inspectMediaManifest(pending.url)
      .then((rows) => {
        if (disposed) return;
        setVariants(rows);
        if (rows.length > 0) setSelectedUrl(rows[0].url);
      })
      .catch((cause) => {
        if (!disposed) setManifestMessage(asIpcError(cause).message);
      });
    return () => { disposed = true; };
  }, [pending.url, pending.kind, pending.id]);

  const host = (() => {
    try { return new URL(pending.url).host; } catch { return "unknown source"; }
  })();
  const kindLabel = (() => {
    if (contentType.startsWith("video/")) return "Video";
    if (contentType.startsWith("audio/")) return "Audio";
    if (contentType.includes("pdf") || contentType.startsWith("text/")) return "Document";
    if (contentType.includes("zip") || contentType.includes("compressed")) return "Compressed";
    if (contentType.includes("executable") || contentType.includes("msi")) return "Program";
    return "Other";
  })();

  async function resolve(action: "start" | "queue" | "cancel") {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await api.resolveBrowserConfirmation({
        id: pending.id,
        action,
        url: selectedUrl || undefined,
        suggested_name: name.trim() || undefined,
        destination_directory: destination.trim() || undefined,
        category_id: category || undefined,
        queue_id: queue || undefined,
      });
      onResolved();
      onClose();
    } catch (cause) {
      setError(asIpcError(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      title="Confirm download"
      description="A link from your browser is waiting for your decision."
      onClose={onClose}
      busy={busy}
      wide
    >
      <div className="browser-confirm-body">
        <InfoBanner>
          <strong>{host}</strong> asked SwiftFetch to download a file. Nothing starts until you choose
          Start download or Add to queue.
        </InfoBanner>

        <div className="browser-confirm-grid">
          <div className="browser-confirm-main">
            <label className="field">
              File name
              <input value={name} onChange={(event) => setName(event.target.value)} aria-label="File name" />
              <span className="field-hint">You can rename the file before it is saved.</span>
            </label>
            <label className="field">
              Destination folder
              <input value={destination} onChange={(event) => setDestination(event.target.value)} placeholder="Use the default download folder" aria-label="Destination folder" />
              <span className="field-hint">Leave blank to use the current SwiftFetch default.</span>
            </label>
            <div className="browser-confirm-facts">
              <div><span>Type</span><strong className="browser-kind"><Icon name="download" size={15} /> {kindLabel}</strong></div>
              <div><span>Size</span><strong>{probing ? "Checking…" : size !== null ? formatBytes(size) : "Not reported"}</strong></div>
              <div><span>Origin</span><strong className="break-anywhere">{host}</strong></div>
            </div>
            {variants.length > 0 && (
              <label className="field">
                Available version
                <select value={selectedUrl || ""} onChange={(event) => setSelectedUrl(event.target.value)} aria-label="Available version">
                  {variants.map((variant) => <option key={variant.url} value={variant.url}>{variant.label}</option>)}
                </select>
                <span className="field-hint">Only directly addressable variants are offered. Protected streams are never decrypted.</span>
              </label>
            )}
            {manifestMessage && <p className="field-hint">{manifestMessage}</p>}
          </div>
          <div className="browser-confirm-side">
            <label className="field">
              Category
              <select value={category} onChange={(event) => setCategory(event.target.value)} aria-label="Category">
                <option value="">Automatic / default</option>
                {categories.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
              </select>
            </label>
            <label className="field">
              Queue
              <select value={queue} onChange={(event) => setQueue(event.target.value)} aria-label="Queue">
                <option value="">Default queue</option>
                {queues.filter((item) => item.enabled).map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
              </select>
            </label>
            <p className="field-hint">Existing files are never replaced automatically. SwiftFetch keeps both copies.</p>
          </div>
        </div>

        <ErrorBanner error={error} />
      </div>

      <footer className="dialog-footer browser-confirm-actions">
        <button type="button" className="button" disabled={busy} onClick={() => void resolve("cancel")}>Cancel</button>
        <button type="button" className="button" disabled={busy} onClick={() => void resolve("queue")}>Add to queue</button>
        <button type="button" className="button button-primary" disabled={busy} onClick={() => void resolve("start")}>
          {busy ? "Working…" : "Start download"}
        </button>
      </footer>
    </Dialog>
  );
}
