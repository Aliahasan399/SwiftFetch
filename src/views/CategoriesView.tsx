import { useCallback, useRef, useState, type FormEvent } from "react";
import { useResource } from "../hooks";
import { api, asIpcError, IpcError } from "../ipc";
import { downloadStore, useDownloads } from "../store";
import type { CategoryDto } from "../types";
import { Dialog } from "../components/Dialog";
import { EmptyState } from "../components/EmptyState";
import { ErrorBanner, InfoBanner, LoadingState } from "../components/Feedback";
import { Icon } from "../components/Icon";
import { PAGE_SIZE, Pagination } from "../components/Pagination";
import { useTaskActions } from "../components/TaskActionProvider";
import "../components/Forms.css";

function extensionSummary(raw: string): string[] {
  try { const value: unknown = JSON.parse(raw); return Array.isArray(value) && value.every((entry) => typeof entry === "string") ? value : ["Custom rules"]; }
  catch { return ["Invalid saved JSON"]; }
}

function CategoryEditor({ category, onClose, onSave }: { category: CategoryDto | null; onClose: () => void; onSave: (category: CategoryDto) => Promise<void> }) {
  const [draft, setDraft] = useState<CategoryDto>(() => category ? { ...category } : { id: crypto.randomUUID(), name: "", destination_template: "", extension_rules_json: "[]" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const saving = useRef(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (saving.current) return;
    setError(null);
    try {
      if (!draft.name.trim() || !draft.destination_template.trim()) throw new IpcError({ code: "INVALID_CATEGORY", message: "A category name and destination template are required.", retryable: false });
      let parsed: unknown;
      try { parsed = JSON.parse(draft.extension_rules_json); }
      catch { throw new IpcError({ code: "INVALID_CATEGORY", message: "Extension rules must be valid JSON, for example [\"zip\", \"7z\"].", retryable: false }); }
      if (!Array.isArray(parsed) || !parsed.every((entry) => typeof entry === "string" && entry.trim().length > 0)) throw new IpcError({ code: "INVALID_CATEGORY", message: "Use a JSON array of non-empty extension strings, or [] for no extension rules.", retryable: false });
      saving.current = true; setBusy(true);
      await onSave(draft); onClose();
    } catch (cause) { setError(asIpcError(cause)); }
    finally { saving.current = false; setBusy(false); }
  }
  return <Dialog title={category ? "Edit category" : "Create category"} description="Organize new downloads with destination and extension rules." onClose={onClose} busy={busy}>
    <form onSubmit={(event) => { void submit(event); }}><div className="dialog-body form-stack"><ErrorBanner error={error} /><fieldset className="fieldset-reset form-stack" disabled={busy}>
      <label className="field">Category name<input required data-autofocus maxLength={120} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} placeholder="Archives" /></label>
      <label className="field">Destination template<input required value={draft.destination_template} onChange={(event) => setDraft({ ...draft, destination_template: event.target.value })} placeholder="%USERPROFILE%/Downloads/Archives" /><span className="field-hint">The template is passed unchanged; the core owns expansion and path validation.</span></label>
      <label className="field">Extension rules (JSON)<textarea className="mono" required rows={4} spellCheck={false} value={draft.extension_rules_json} onChange={(event) => setDraft({ ...draft, extension_rules_json: event.target.value })} placeholder={'["zip", "7z", "tar"]'} /><span className="field-hint">A JSON array of extensions, such as ["zip", "7z"]. Existing tasks are not moved.</span></label>
    </fieldset></div><footer className="dialog-footer"><button type="button" className="button" disabled={busy} onClick={onClose}>Cancel</button><button type="submit" className="button button-primary" disabled={busy}>{busy ? "Saving…" : "Save category"}</button></footer></form>
  </Dialog>;
}

export function CategoriesView() {
  const revision = useDownloads().catalog_revision;
  const load = useCallback(() => { void revision; return api.listCategories(); }, [revision]);
  const categories = useResource(load);
  const actions = useTaskActions();
  const [editor, setEditor] = useState<{ category: CategoryDto | null } | null>(null);
  const [page, setPage] = useState(1);
  const entries = categories.data ?? [];
  const safePage = Math.min(page, Math.max(1, Math.ceil(entries.length / PAGE_SIZE)));
  async function save(category: CategoryDto) {
    await api.saveCategory(category);
    const confirmed = await api.listCategories();
    const saved = confirmed.find((entry) => entry.id === category.id);
    if (!saved || (Object.keys(category) as (keyof CategoryDto)[]).some((key) => category[key] !== saved[key])) throw new IpcError({ code: "READBACK_FAILED", message: "The category was not returned as saved. Reload categories before trying again.", retryable: true });
    categories.replace(confirmed); downloadStore.bumpCatalog();
    actions.notify("Category saved and confirmed with the core.");
  }
  return <div className="view-stack"><InfoBanner>Category rules apply to new tasks according to core policy. Editing a category never moves or renames an existing downloaded file. Category deletion is not in the IPC contract.</InfoBanner><ErrorBanner error={categories.error} onRetry={() => { void categories.reload(); }} />
    <section className="panel"><header className="panel-header"><div><h2>Your categories</h2><p>A home for every type of download.</p></div><button className="button button-primary" onClick={() => setEditor({ category: null })}><Icon name="plus" size={18} />Create category</button></header>
      {categories.loading && !categories.data ? <LoadingState /> : entries.length === 0 ? <EmptyState icon="folder" title={categories.error ? "Categories unavailable" : "No categories yet"} description="Create a category to assign a destination and file extension rules." /> : <div className="category-grid">{entries.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE).map((category) => { const extensions = extensionSummary(category.extension_rules_json); return <article className="category-card" key={category.id}><div className="category-card-header"><span className="section-icon"><Icon name="folder" size={24} /></span><button className="icon-button" aria-label={`Edit ${category.name}`} title="Edit category" onClick={() => setEditor({ category })}><Icon name="edit" size={17} /></button></div><h3>{category.name}</h3><p className="break-anywhere mono category-path">{category.destination_template}</p><div className="extension-tags">{extensions.length === 0 ? <span className="muted">No extension rules</span> : extensions.slice(0, 12).map((extension, index) => <span className="tag" key={`${index}-${extension}`}>{extension}</span>)}{extensions.length > 12 && <span className="muted">{extensions.length - 12} more in editor</span>}</div></article>; })}</div>}
      {entries.length > 0 && <Pagination page={safePage} total={entries.length} onChange={setPage} />}
    </section>{editor && <CategoryEditor category={editor.category} onSave={save} onClose={() => setEditor(null)} />}
  </div>;
}
