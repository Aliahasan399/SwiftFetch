import { useEffect, useId, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useT } from "../i18n";
import { Icon } from "./Icon";
import "./Dialog.css";

export function Dialog({ title, description, children, onClose, busy = false, wide = false }: { title: string; description?: string; children: ReactNode; onClose: () => void; busy?: boolean; wide?: boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  const t = useT();
  useEffect(() => {
    const element = ref.current;
    const previous = document.activeElement;
    if (!element) return;
    if (!element.open) element.showModal();
    const target = element.querySelector<HTMLElement>("[data-autofocus]") ?? element.querySelector<HTMLElement>("button, input, select, textarea");
    target?.focus();
    return () => {
      if (element.open) element.close();
      // Return focus to the trigger. If the trigger was unmounted by the very
      // action the dialog confirmed, fall back to the main landmark instead of
      // dropping focus on <body>.
      if (previous instanceof HTMLElement && previous.isConnected && previous !== document.body) previous.focus();
      else document.getElementById("main-content")?.focus();
    };
  }, []);

  return createPortal(<dialog ref={ref} className={`dialog ${wide ? "dialog-wide" : ""}`} aria-labelledby={titleId} aria-describedby={description ? descriptionId : undefined} aria-modal="true" aria-busy={busy} onCancel={(event) => { event.preventDefault(); if (!busy) onClose(); }} onKeyDown={(event) => {
    if (event.key !== "Tab") return;
    const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]')).filter((element) => element.getClientRects().length > 0);
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (!first) { event.preventDefault(); event.currentTarget.focus(); return; }
    if (event.shiftKey && (document.activeElement === first || document.activeElement === event.currentTarget)) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  }}>
    <header className="dialog-header"><div><h2 id={titleId}>{title}</h2>{description && <p id={descriptionId}>{description}</p>}</div><button type="button" className="icon-button" aria-label={t("dialog.closeDialog")} title={`${t("dialog.closeDialog")} (Escape)`} onClick={onClose} disabled={busy}><Icon name="close" /></button></header>
    {children}
  </dialog>, document.body);
}
