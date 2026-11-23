import type { PendingConfirmation } from "./types";

/**
 * Choose the confirmation that should be visible. Later browser handoffs must
 * never be dropped simply because a dialog was open when they arrived: once
 * the current dialog closes, the next pending row is selected immediately.
 */
export function selectPendingId(
  rows: PendingConfirmation[],
  currentId: string | null,
  dialogOpen: boolean,
): string | null {
  if (currentId && rows.some((row) => row.id === currentId)) return currentId;
  if (dialogOpen) return null;
  return rows.length > 0 ? rows[0].id : null;
}
