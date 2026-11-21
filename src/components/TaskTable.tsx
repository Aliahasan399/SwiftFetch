import { useState } from "react";
import type { TaskSummary } from "../types";
import { EmptyState } from "./EmptyState";
import { Pagination, PAGE_SIZE } from "./Pagination";
import { useTaskActions } from "./TaskActionProvider";
import { TaskRow } from "./TaskRow";

export function TaskTable({ tasks, onDetails, showPriority = false, selected, onSelect, onPriorityChange, onLoadMore, hasMore = false, emptyTitle = "No downloads here", emptyDescription = "Add a download to get started.", caption = "Downloads" }: { tasks: readonly TaskSummary[]; onDetails: (id: string) => void; showPriority?: boolean; selected?: ReadonlySet<string>; onSelect?: (id: string, selected: boolean) => void; onPriorityChange?: (id: string, priority: number) => void; onLoadMore?: () => void; hasMore?: boolean; emptyTitle?: string; emptyDescription?: string; caption?: string }) {
  const [page, setPage] = useState(1);
  const actions = useTaskActions();
  const safePage = Math.min(page, Math.max(1, Math.ceil(tasks.length / PAGE_SIZE)));
  const visible = tasks.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);
  const allSelected = visible.length > 0 && visible.every((task) => selected?.has(task.id));
  if (!tasks.length) return <EmptyState title={emptyTitle} description={emptyDescription} />;
  return <>
    <div className="table-scroll" tabIndex={0} role="region" aria-label={`${caption}, horizontally scrollable`}>
      <table className="task-table"><caption className="sr-only">{caption}</caption><thead><tr>
        {onSelect && <th className="selection-cell"><input type="checkbox" aria-label="Select all tasks on this page" checked={allSelected} onChange={(event) => visible.forEach((task) => onSelect(task.id, event.target.checked))} /></th>}
        <th scope="col">File</th><th scope="col">Status & progress</th><th scope="col">Speed / ETA</th>{showPriority && <th scope="col">Priority</th>}<th scope="col">Actions</th>
      </tr></thead><tbody>{visible.map((task) => <TaskRow key={task.id} task={task} onDetails={onDetails} onAction={actions.request} busy={actions.busyIds.has(task.id)} showPriority={showPriority} selected={selected?.has(task.id)} onSelect={onSelect} onPriorityChange={onPriorityChange} />)}</tbody></table>
    </div>
    <Pagination page={safePage} total={tasks.length} onChange={setPage} />
    {hasMore && onLoadMore && <button className="button button-small" onClick={onLoadMore} disabled={false}>Load more</button>}
  </>;
}
