import { formatNumber } from "../format";

export const PAGE_SIZE = 25;

export function Pagination({ page, total, pageSize = PAGE_SIZE, onChange }: { page: number; total: number; pageSize?: number; onChange: (page: number) => void }) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const current = Math.min(page, pages);
  return <nav className="pagination" aria-label="List pagination">
    <span>{total === 0 ? "No items" : `${formatNumber((current - 1) * pageSize + 1)}–${formatNumber(Math.min(current * pageSize, total))} of ${formatNumber(total)}`}</span>
    <div><button className="button button-small" disabled={current <= 1} onClick={() => onChange(current - 1)}>Previous</button><span>Page {current} of {pages}</span><button className="button button-small" disabled={current >= pages} onClick={() => onChange(current + 1)}>Next</button></div>
  </nav>;
}
