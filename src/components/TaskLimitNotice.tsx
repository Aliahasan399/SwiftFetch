import { formatNumber } from "../format";
import { loadMoreTasks, MAX_TASK_LIMIT, useDownloads } from "../store";
import { InfoBanner } from "./Feedback";

export function TaskLimitNotice() {
  const state = useDownloads();
  if (!state.limit_reached) return null;
  const atCap = state.tasks.length >= MAX_TASK_LIMIT;
  return <InfoBanner><p>This view is based on {formatNumber(state.tasks.length)} loaded tasks. Counts and search cover loaded tasks only; older tasks may be omitted.</p>{atCap ? <p>The UI cap of {formatNumber(MAX_TASK_LIMIT)} loaded tasks has been reached. Narrow the filters or clear history to see older records.</p> : state.has_more ? <button className="button button-small" disabled={state.loading} onClick={() => { void loadMoreTasks().catch(() => undefined); }}>Load more tasks</button> : null}</InfoBanner>;
}
