import { useResource } from "../hooks";
import { api } from "../ipc";
import type { Settings } from "../types";
import { ErrorBanner, LoadingState } from "../components/Feedback";
import { SettingsForm } from "../components/SettingsForm";

export function SettingsView({ onSaved }: { onSaved: (settings: Settings) => void }) {
  const resource = useResource(api.getSettings);
  return <div className="view-stack"><ErrorBanner error={resource.error} onRetry={() => { void resource.reload(); }} />{resource.loading && !resource.data && <LoadingState label="Loading your saved preferences…" />}{resource.data && <SettingsForm settings={resource.data} onSaved={(settings) => { resource.replace(settings); onSaved(settings); }} />}</div>;
}
