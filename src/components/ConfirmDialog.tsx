import type { ReactNode } from "react";
import type { IpcError } from "../ipc";
import { Dialog } from "./Dialog";
import { ErrorBanner } from "./Feedback";

export function ConfirmDialog({ title, children, confirmLabel = "Confirm", onConfirm, onClose, busy = false, danger = false, error = null }: { title: string; children: ReactNode; confirmLabel?: string; onConfirm: () => void; onClose: () => void; busy?: boolean; danger?: boolean; error?: IpcError | null }) {
  return <Dialog title={title} onClose={onClose} busy={busy}>
    <div className="dialog-body"><div className="confirm-copy">{children}</div><ErrorBanner error={error} /></div>
    <footer className="dialog-footer"><button type="button" className="button" data-autofocus disabled={busy} onClick={onClose}>Go back</button><button type="button" className={`button ${danger ? "button-danger" : "button-primary"}`} disabled={busy} onClick={onConfirm}>{busy ? "Working…" : confirmLabel}</button></footer>
  </Dialog>;
}
