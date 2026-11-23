// Dependency-free regression runner: Node assertions + the project's installed Vite/TypeScript.
// All IPC responses below are explicitly synthetic test fixtures. No native core is contacted.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { webcrypto } from "node:crypto";
import { createServer } from "vite";
import ts from "typescript";
import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

const server = await createServer({ configFile: false, root: process.cwd(), cacheDir: "src/testing/.vite", logLevel: "error", server: { middlewareMode: true, hmr: false, watch: null }, optimizeDeps: { noDiscovery: true } });
globalThis.window = { crypto: webcrypto };
globalThis.isTauri = true;
const tests = [];
const test = (name, run) => tests.push({ name, run });
let failed = 0;

try {
  const format = await server.ssrLoadModule("/src/format.ts");
  const { api, IpcError, subscribeCore } = await server.ssrLoadModule("/src/ipc.ts");
  const { createDownloadStore, toTaskSummary, refreshTasks, refreshTasksAfterMutation, downloadStore, loadMoreTasks, resetTaskFilter } = await server.ssrLoadModule("/src/store.ts");
  const { availableActions, needsRestartConfirmation, buildControlRequest, normalizePriority } = await server.ssrLoadModule("/src/actions.ts");
  const task = {
    id: "fixture-task", display_name: "fixture.zip", status: "downloading", bytes_received: 10,
    total_bytes: 100, bytes_per_second: 20, eta_seconds: 5, queue_id: "default", category_id: null,
    priority: 50, can_resume: true, error_code: null, error_message: null,
    original_url: "https://example.test/fixture.zip?private=test-fixture", final_url: null,
    destination_path: "C:/fixtures/fixture.zip", temporary_path: "C:/fixtures/fixture.zip.part",
    content_type: "application/zip", checksum_algorithm: "sha256", checksum_expected: null,
    checksum_actual: null, checksum_verified: false, retry_count: 0, max_retries: 3, connections: 4,
    created_at: "2026-01-01T00:00:00Z", updated_at: "2026-01-01T00:00:00Z", completed_at: null,
    segment_count: 4, completed_segments: 0,
  };
  const progress = (patch = {}) => ({ task_id: task.id, status: "downloading", received_bytes: 60, total_bytes: 100, bytes_per_second: 30, eta_seconds: 2, active_connections: 2, segment_done: 2, segment_total: 4, ...patch });
  const storeWithTask = () => { const store = createDownloadStore(); store.replace([task], store.getRevision()); return store; };
  const settings = {
    destination_root: "C:/fixtures", max_active_tasks: 3, connections_per_task: 4,
    global_speed_limit: 0, default_task_speed_limit: 0, max_retries: 3, overwrite_policy: "rename",
    verify_checksums: true, scan_with_antivirus: false, scanner_command: "", prevent_sleep: false,
    start_on_login: false, theme: "dark", retention_days: 90, browser_integration: false,
    proxy: "", scheduler_enabled: true, shutdown_after_complete: false, keep_partial_on_failure: true,
    encrypt_saved_headers: false,
  };

  test("DownloadProjection and Settings fields/nullability mirror Rust", async () => {
    const source = await readFile("src/types.ts", "utf8");
    const ast = ts.createSourceFile("types.ts", source, ts.ScriptTarget.Latest, true);
    for (const [name, path] of [["DownloadProjection", "src-tauri/src/domain.rs"], ["Settings", "src-tauri/src/settings.rs"]]) {
      const rust = await readFile(path, "utf8");
      const body = rust.match(new RegExp(`pub struct ${name} \\{([\\s\\S]*?)\\n\\}`))?.[1];
      assert.ok(body, `${name} Rust struct exists`);
      const rustFields = [...body.matchAll(/pub (\w+): ([^,\n]+),/g)].map((match) => ({ name: match[1], type: match[2] }));
      const declaration = ast.statements.find((node) => ts.isInterfaceDeclaration(node) && node.name.text === name);
      assert.ok(declaration);
      const members = declaration.members.map((member) => ({ name: member.name.getText(ast), type: member.type.getText(ast) }));
      assert.deepEqual(members.map((member) => member.name), rustFields.map((field) => field.name));
      for (const field of rustFields) {
        const type = members.find((member) => member.name === field.name).type;
        assert.equal(type.includes("null"), field.type.startsWith("Option<"), `${name}.${field.name} nullability`);
      }
    }
  });
  test("all Settings fields have a form control", async () => {
    const source = await readFile("src/components/SettingsForm.tsx", "utf8");
    for (const key of Object.keys(settings)) assert.ok(source.includes(`\"${key}\"`), `control for ${key}`);
  });
  test("byte, ETA and date formatting handle zero/unknown/invalid values", () => {
    assert.equal(format.formatBytes(0), "0 B"); assert.equal(format.formatBytes(1024), "1 KiB");
    assert.equal(format.formatBytes(null), "Unknown size"); assert.equal(format.formatBytes(-1), "Unknown size");
    assert.equal(format.formatEta(null), "Estimating…"); assert.equal(format.formatEta(61), "1m 1s");
    assert.equal(format.formatDate("invalid"), "Unknown date");
  });
  test("URL validation rejects unsafe schemes, credentials, controls and oversized input", () => {
    assert.equal(format.validateDownloadUrl("https://example.test/file.zip"), null);
    for (const url of ["", "file:///fixture.zip", "javascript:alert(1)", "https://fixture-user:fixture-password@example.test/f", "https://exa\nmple.test/f", "https://example.test/" + "a".repeat(2050)]) assert.ok(format.validateDownloadUrl(url));
  });
  test("Windows filename validation rejects traversal and reserved names", () => {
    for (const name of ["../file.zip", "CON", "lpt1.txt", "file. ", "bad?.zip"]) assert.ok(format.validateFilename(name));
    assert.equal(format.validateFilename("archive.tar.gz"), null);
  });
  test("URL and error display omit sensitive values", () => {
    const redacted = format.safeUrl("https://fixture-user:fixture-password@example.test/file?token=fixture#fragment");
    assert.equal(redacted, "https://example.test/file?[query hidden]");
    const message = format.redactSensitiveText("Authorization: fixture-value\nCookie: fixture-cookie\nhttps://example.test/?token=fixture C:/fixtures/file.zip");
    assert.ok(!message.includes("fixture-value") && !message.includes("fixture-cookie") && !message.includes("?token="));
  });
  test("100% bytes does not imply a completed status", () => {
    assert.equal(format.progressPercent(100, 100, "downloading"), 100);
    assert.match(format.progressText({ bytes_received: 100, total_bytes: 100, status: "downloading" }), /awaiting core finalization/);
    const store = storeWithTask(); store.progress(progress({ received_bytes: 100 })); store.flushProgress();
    assert.equal(store.getSnapshot().tasks[0].status, "downloading"); store.stopProgress();
  });
  test("event field names map correctly and connection count stays separate", () => {
    const store = storeWithTask(); store.progress(progress()); store.flushProgress();
    const snapshot = store.getSnapshot(); const row = snapshot.tasks[0];
    assert.equal(row.bytes_received, 60); assert.equal(row.completed_segments, 2); assert.equal(row.segment_count, 4);
    assert.equal(row.connections, 4); assert.equal(snapshot.active_connections[task.id], 2); store.stopProgress();
  });
  test("global summaries omit URL, path and hash values", () => {
    const summary = toTaskSummary(task);
    for (const field of ["original_url", "final_url", "destination_path", "temporary_path", "checksum_expected", "checksum_actual"]) assert.equal(field in summary, false);
  });
  test("paused status discards buffered progress and resets speed", () => {
    const store = storeWithTask(); store.progress(progress()); store.status({ task_id: task.id, status: "paused" }); store.flushProgress();
    assert.equal(store.getSnapshot().tasks[0].status, "paused"); assert.equal(store.getSnapshot().tasks[0].bytes_per_second, 0); store.stopProgress();
  });
  test("a delayed progress event cannot resurrect a completed task", () => {
    const store = storeWithTask(); store.status({ task_id: task.id, status: "completed" }); store.progress(progress()); store.flushProgress();
    assert.equal(store.getSnapshot().tasks[0].status, "completed"); store.stopProgress();
  });
  test("a stale list response cannot overwrite a newer progress event", () => {
    const store = storeWithTask(); const before = store.getRevision(); store.progress(progress()); store.replace([task], before);
    assert.equal(store.getSnapshot().tasks[0].bytes_received, 60); store.stopProgress();
  });
  test("a stale list response cannot resurrect removed metadata", () => {
    const store = storeWithTask(); const before = store.getRevision(); store.remove(task.id); store.replace([task], before);
    assert.equal(store.getSnapshot().tasks.length, 0); store.stopProgress();
  });
  test("an unknown status event does not hide a newly loaded task", () => {
    const store = createDownloadStore(); const before = store.getRevision();
    assert.equal(store.status({ task_id: task.id, status: "downloading" }), false);
    store.replace([task], before); assert.equal(store.getSnapshot().tasks.length, 1); store.stopProgress();
  });
  test("manual progress flush cancels its pending timer", () => {
    const originalSet = globalThis.setTimeout; const originalClear = globalThis.clearTimeout;
    let cleared = 0; globalThis.setTimeout = () => 123; globalThis.clearTimeout = () => { cleared += 1; };
    try { const store = storeWithTask(); store.progress(progress()); store.flushProgress(); assert.equal(cleared, 1); store.stopProgress(); }
    finally { globalThis.setTimeout = originalSet; globalThis.clearTimeout = originalClear; }
  });
  test("buffered events preceding a read do not override its fresher metadata", () => {
    const store = storeWithTask(); store.progress(progress()); const before = store.getRevision();
    store.replace([{ ...task, status: "completed", bytes_received: 100, checksum_verified: true }], before);
    assert.equal(store.getSnapshot().tasks[0].status, "completed"); assert.equal(store.getSnapshot().tasks[0].checksum_verified, true); store.stopProgress();
  });
  test("new progress preserves live bytes while accepting fresh read metadata", () => {
    const store = storeWithTask(); const before = store.getRevision(); store.progress(progress());
    store.replace([{ ...task, can_resume: false, connections: 8 }], before);
    const current = store.getSnapshot().tasks[0]; assert.equal(current.bytes_received, 60); assert.equal(current.connections, 8); assert.equal(current.can_resume, false); store.stopProgress();
  });
  test("an older get response cannot roll back a newer list snapshot", () => {
    const store = storeWithTask(); const oldGet = store.getRevision(); const listRead = store.getRevision();
    store.replace([{ ...task, status: "completed", checksum_verified: true }], listRead);
    store.upsert(task, oldGet); assert.equal(store.getSnapshot().tasks[0].status, "completed"); store.stopProgress();
  });
  test("all documented failure modes have actionable copy", () => {
    for (const code of ["NETWORK_ERROR", "DISK_FULL", "SERVER_DOES_NOT_SUPPORT_RANGES", "RESUME_INCOMPATIBLE", "ANTIVIRUS_UNAVAILABLE", "BROWSER_BRIDGE_UNAVAILABLE"]) assert.ok(format.errorGuidance(code));
  });
  test("task controls respect terminal state and restart confirmation", () => {
    assert.ok(!availableActions("completed").includes("pause")); assert.ok(availableActions("failed").includes("retry"));
    assert.equal(needsRestartConfirmation({ ...task, can_resume: false }, "resume"), true);
    assert.equal(needsRestartConfirmation(task, "pause"), false);
  });
  test("schedule weekday masks use Monday bit 0 and Sunday bit 6", () => {
    assert.equal(format.formatDays(1), "Mon"); assert.equal(format.formatDays(64), "Sun"); assert.equal(format.formatDays(127), "Every day");
  });
  test("commands use Tauri argument names and snake_case DTO fields", async () => {
    const calls = []; mockIPC((command, payload) => { calls.push({ command, payload }); return { ok: true, data: null }; });
    const add = { url: "https://example.test/file.zip", headers: [], connections: 4, start_policy: "manual" };
    const category = { id: "fixture-category", name: "Fixtures", destination_template: "C:/fixtures", extension_rules_json: "[]" };
    const schedule = { id: "fixture-schedule", queue_id: "default", days_mask: 127, start_local_time: "22:00", end_local_time: "06:00", action_after: "nothing", enabled: true };
    await api.addDownload(add); await api.controlDownload({ task_id: task.id, action: "pause", delete_file: true });
    await api.listTasks(42); await api.getTask(task.id); await api.getSettings(); await api.saveSettings(settings);
    await api.listCategories(); await api.saveCategory(category); await api.listQueues(); await api.listSchedules();
    await api.saveSchedule(schedule); await api.deleteSchedule(schedule.id); await api.getDiagnostics(); await api.clearHistory();
    // Priority edit uses the same control_download command with action set_priority
    await api.controlDownload({ task_id: task.id, action: "set_priority", priority: 10, delete_file: false });
    assert.deepEqual(calls.map((call) => call.command), ["add_download", "control_download", "list_tasks", "get_task", "get_settings", "save_settings", "list_categories", "save_category", "list_queues", "list_schedules", "save_schedule", "delete_schedule", "get_diagnostics", "clear_history", "control_download"]);
    assert.deepEqual(calls[0].payload, { req: add });
    assert.deepEqual(calls[1].payload, { req: { task_id: task.id, action: "pause", delete_file: false } });
    assert.deepEqual(calls[2].payload, { limit: 42 }); assert.deepEqual(calls[3].payload, { taskId: task.id });
    assert.deepEqual(calls[5].payload, { settings });
    assert.deepEqual(calls[7].payload, { cat: category });
    assert.deepEqual(calls[10].payload, { schedule });
    assert.deepEqual(calls[14].payload, { req: { task_id: task.id, action: "set_priority", priority: 10, delete_file: false } });
  });

  test("set_priority is an available action for all non-terminal statuses", () => {
    for (const status of ["queued", "starting", "downloading", "paused", "retry_wait", "verifying", "scanning", "failed", "cancelled"]) {
      assert.ok(availableActions(status).includes("set_priority"), `set_priority available for ${status}`);
    }
    assert.ok(!availableActions("completed").includes("pause"));
  });
  test("post-mutation refresh does not reuse a pre-mutation poll", async () => {
    let resolveOld; let count = 0;
    mockIPC(() => { count += 1; return count === 1 ? new Promise((resolve) => { resolveOld = resolve; }) : { ok: true, data: { tasks: [], has_more: false } }; });
    const oldPoll = refreshTasks(); const readBack = refreshTasksAfterMutation();
    resolveOld({ ok: true, data: { tasks: [{ ...task, status: "completed" }], has_more: false } });
    await Promise.all([oldPoll, readBack]);
    assert.equal(count, 2); assert.equal(downloadStore.getSnapshot().tasks.length, 0);
  });
  test("keyset pagination reloads the whole prefix and removes cleared history", async () => {
    let rows = Array.from({ length: 240 }, (_, index) => ({ ...task, id: `page-${String(240 - index).padStart(3, "0")}`, status: "completed" }));
    const calls = [];
    mockIPC((command, args) => {
      assert.equal(command, "list_tasks");
      calls.push(args);
      assert.ok(args.limit <= 500);
      const candidates = rows.filter((row) => !args.cursor || `${row.updated_at}|${row.id}` < args.cursor);
      return { ok: true, data: { tasks: candidates.slice(0, args.limit), has_more: candidates.length > args.limit } };
    });
    await refreshTasks(null);
    assert.equal(downloadStore.getSnapshot().tasks.length, 200);
    assert.equal(downloadStore.getSnapshot().has_more, true);
    await loadMoreTasks();
    assert.equal(downloadStore.getSnapshot().tasks.length, 240);
    assert.ok(calls.some((call) => call.cursor));
    assert.equal(downloadStore.getSnapshot().limit_reached, false);
    rows = [];
    await refreshTasksAfterMutation();
    assert.equal(downloadStore.getSnapshot().tasks.length, 0);
    assert.equal(downloadStore.getSnapshot().has_more, false);
  });
  test("leaving a filtered history restores the complete task catalog", async () => {
    mockIPC((command, args) => ({ ok: true, data: { tasks: args.status ? [] : [task], has_more: false } }));
    await refreshTasks("completed");
    assert.equal(downloadStore.getSnapshot().status_filter, "completed");
    await resetTaskFilter();
    assert.equal(downloadStore.getSnapshot().status_filter, null);
    assert.equal(downloadStore.getSnapshot().tasks.length, 1);
  });
  test("priority updates send the selected value across the supported range", () => {
    assert.deepEqual(buildControlRequest(task, "set_priority", 100), { task_id: task.id, action: "set_priority", priority: 100, delete_file: false });
    assert.equal(normalizePriority(-1), 0);
    assert.equal(normalizePriority(101), 100);
  });
  test("desktop recovery and update wrappers keep the registered command payloads", async () => {
    const calls=[];
    mockIPC((command,payload)=>{calls.push({command,payload});return {ok:true,data:null};});
    await api.readClipboardUrl();await api.reportReady();await api.getPowerRequest();await api.cancelShutdown('request-1');
    await api.getTaskSegments(task.id);await api.getUpdateState();await api.checkForUpdates();await api.installUpdate();await api.rollbackUpdate();
    await api.exportQueue();await api.backupMetadata();await api.importQueue('{"protocol":1}');
    assert.deepEqual(calls.map(call=>call.command),['read_clipboard_url','report_ready','get_power_request','cancel_shutdown','get_task_segments','get_update_state','check_for_updates','install_update','rollback_update','export_queue','backup_metadata','import_queue']);
    assert.deepEqual(calls[3].payload,{requestId:'request-1'});
    assert.deepEqual(calls[4].payload,{taskId:task.id});
    assert.deepEqual(calls[11].payload,{metadata:'{"protocol":1}'});
  });
  test("recovery commands bind camelCase backup names with explicit confirmation", async () => {
    const calls = [];
    mockIPC((command, payload) => { calls.push({ command, payload }); return { ok: true, data: null }; });
    await api.getRecoveryState();
    await api.restoreMetadata("queue-2026.sqlite");
    await api.restartApplication();
    await api.getDiskSpace();
    await api.getDiskSpace("C:/fixtures");
    assert.deepEqual(calls.map((call) => call.command), [
      "get_recovery_state", "restore_metadata", "restart_application", "get_disk_space", "get_disk_space",
    ]);
    assert.deepEqual(calls[1].payload, { backupName: "queue-2026.sqlite", confirm: true });
    assert.deepEqual(calls[2].payload, {});
    assert.deepEqual(calls[3].payload, {});
    assert.deepEqual(calls[4].payload, { path: "C:/fixtures" });
  });
  test("recovery and disk DTOs keep snake_case fields and null availability", async () => {
    const source = await readFile("src/types.ts", "utf8");
    for (const [name, fields] of [
      ["RecoveryState", ["message", "data_directory", "backups"]],
      ["BackupInfo", ["name", "size_bytes"]],
      ["RestoreResult", ["restored_tasks", "safety_backup", "restart_required"]],
      ["DiskSpace", ["available_bytes", "total_bytes"]],
    ]) {
      const body = source.match(new RegExp(`interface ${name} \\{([\\s\\S]*?)\\n\\}`))?.[1];
      assert.ok(body, `${name} interface exists`);
      for (const field of fields) assert.ok(body.includes(`${field}:`), `${name}.${field}`);
    }
    mockIPC(() => ({ ok: true, data: null }));
    assert.equal(await api.getDiskSpace(), null);
    const space = { available_bytes: 1024, total_bytes: 4096 };
    mockIPC(() => ({ ok: true, data: space }));
    assert.deepEqual(await api.getDiskSpace(), space);
    const restored = { restored_tasks: 3, safety_backup: "C:/data/backups/before-recovery", restart_required: true };
    mockIPC(() => ({ ok: true, data: restored }));
    assert.deepEqual(await api.restoreMetadata("queue.sqlite"), restored);
  });
  test("explicit collision choices use only supported core actions", () => {
    for(const action of ['overwrite','rename','skip'])assert.equal(buildControlRequest(task,action).action,action);
  });
  test("locale dictionaries provide deterministic fallback and matching navigation keys", async () => {
    const {translate,normalizeLocale}=await server.ssrLoadModule('/src/i18n.tsx');
    assert.equal(normalizeLocale('unknown'),'en');
    assert.equal(translate('en','app.add'),'Add download');
    assert.equal(translate('bn','app.add'),'ডাউনলোড যোগ করুন');
    assert.equal(translate('unknown','nav.history'),'History');
  });
  test("update health requires successful task and settings initialization", async () => {
    const {bootDesktop}=await server.ssrLoadModule('/src/desktopBoot.ts');
    let ready=0;const acknowledge=async()=>{ready++;};
    await assert.rejects(bootDesktop(async()=>{throw new Error('queue unavailable');},async()=>settings,()=>{},acknowledge));
    await assert.rejects(bootDesktop(async()=>{},async()=>{throw new Error('settings unavailable');},()=>{},acknowledge));
    assert.equal(ready,0);
    await bootDesktop(async()=>{},async()=>settings,()=>{},acknowledge,()=>false);
    assert.equal(ready,0);
    await bootDesktop(async()=>{},async()=>settings,confirmed=>assert.equal(confirmed,settings),acknowledge);
    assert.equal(ready,1);
  });
  test("command envelopes reject failed and malformed core responses", async () => {
    mockIPC(() => ({ ok: false, error: { code: "DISK_FULL", message: "Not enough space", retryable: false } }));
    await assert.rejects(api.listTasks(), (error) => error instanceof IpcError && error.code === "DISK_FULL");
    mockIPC(() => ({ ok: true })); await assert.rejects(api.listTasks(), (error) => error.code === "INVALID_RESPONSE");
    mockIPC(() => { throw "Command list_tasks not found"; }); await assert.rejects(api.listTasks(), (error) => error.code === "COMMAND_UNAVAILABLE");
  });
  test("browser preview rejects commands without a fabricated backend", async () => {
    globalThis.isTauri = false;
    try { await assert.rejects(api.listTasks(), (error) => error.code === "CORE_UNAVAILABLE"); }
    finally { globalThis.isTauri = true; }
  });
  test("all four typed events are delivered and cleanup removes callbacks", async () => {
    mockIPC(() => null, { shouldMockEvents: true }); const received = [];
    const names = ["download_progress", "download_status", "queue_changed", "recovery"];
    const handlers = Object.fromEntries(names.map((name) => [name, (payload) => received.push({ name, payload })]));
    const stop = await subscribeCore(handlers);
    const payloads = [progress(), { task_id: task.id, status: "paused" }, {}, { paused: 1, restarted: 0 }];
    for (let index = 0; index < names.length; index += 1) await emit(names[index], payloads[index]);
    assert.deepEqual(received.map((event) => event.name), names);
    assert.deepEqual(received.map((event) => event.payload), payloads);
    stop(); await new Promise((resolve) => setImmediate(resolve));
    assert.equal(window.__TAURI_INTERNALS__.callbacks.size, 0);
  });
  test("a partial event-registration failure unregisters successful listeners", async () => {
    const removed = [];
    mockIPC((command, payload) => {
      if (command === "plugin:event|listen") { if (payload.event === "recovery") throw "fixture registration failure"; return payload.handler; }
      if (command === "plugin:event|unlisten") { removed.push(payload.event); return null; }
      return null;
    });
    const noop = () => undefined;
    await assert.rejects(subscribeCore({ download_status: noop, download_progress: noop, queue_changed: noop, recovery: noop }));
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(removed.sort(), ["download_progress", "download_status", "queue_changed"]);
  });

  test("Bengali dictionary has full key parity with English and no placeholders", async () => {
    const { en, bn, translate, normalizeLocale, SUPPORTED_LOCALES } = await server.ssrLoadModule("/src/i18n.tsx");
    assert.deepEqual([...SUPPORTED_LOCALES], ["en", "bn"]);
    const enKeys = Object.keys(en).sort();
    assert.deepEqual(Object.keys(bn).sort(), enKeys, "bn defines exactly the en keys");
    for (const key of enKeys) {
      assert.equal(typeof bn[key], "string", `bn.${key} is a string`);
      assert.ok(bn[key].trim().length > 0, `bn.${key} is not blank`);
      assert.notEqual(bn[key], key, `bn.${key} is not an untranslated key placeholder`);
      assert.equal(translate("bn", key), bn[key], `bn.${key} is used without falling back`);
      assert.equal(translate("unknown", key), en[key], "unknown locales fall back to English");
    }
    assert.equal(normalizeLocale("bn"), "bn");
    assert.notEqual(translate("bn", "dialog.closeDialog"), translate("en", "dialog.closeDialog"), "a11y labels are translated too");
    const dialog = await readFile("src/components/Dialog.tsx", "utf8");
    assert.match(dialog, /aria-label=\{t\("dialog\.closeDialog"\)\}/, "close icon button uses the translated label");
  });
  test("dialog shell uses native modal focus semantics and restores the trigger", async () => {
    const source = await readFile("src/components/Dialog.tsx", "utf8");
    for (const token of ["showModal()", 'aria-modal="true"', "aria-labelledby", "aria-describedby", "onCancel", 'event.key !== "Tab"', "data-autofocus", "previous.focus()", 'getElementById("main-content")?.focus()', "element.close()"]) {
      assert.ok(source.includes(token), `Dialog.tsx includes ${token}`);
    }
    for (const file of ["ConfirmDialog.tsx", "AddDownloadDialog.tsx", "PriorityDialog.tsx", "TaskDetailsDialog.tsx", "RecoveryPanel.tsx", "UpdatePanel.tsx"]) {
      const body = await readFile(`src/components/${file}`, "utf8");
      assert.match(body, /<(Confirm)?Dialog\b/, `${file} uses the shared dialog shell`);
    }
  });
  test("keyboard affordances: focus ring, skip link, labelled icon buttons and tables", async () => {
    const css = await readFile("src/App.css", "utf8");
    assert.match(css, /:focus-visible\s*\{[^}]*outline:/, "global visible focus ring");
    assert.match(css, /\.skip-link:focus\s*\{/, "skip link is revealed on focus");
    const app = await readFile("src/App.tsx", "utf8");
    assert.match(app, /className="skip-link"/);
    assert.match(app, /href="#main-content"/);
    assert.match(app, /id="main-content"[^>]*tabIndex=\{-1\}/);
    const { readdir } = await import("node:fs/promises");
    const files = [];
    for (const dir of ["src", "src/components", "src/views"]) for (const name of await readdir(dir)) if (name.endsWith(".tsx")) files.push(`${dir}/${name}`);
    let iconButtons = 0;
    for (const file of files) {
      const body = await readFile(file, "utf8");
      for (const match of body.matchAll(/<button\b[^>]*>/gs)) {
        if (!match[0].includes("icon-button")) continue;
        iconButtons += 1;
        assert.match(match[0], /aria-label=/, `icon-only button in ${file} has an aria-label`);
      }
    }
    assert.ok(iconButtons >= 6, `found ${iconButtons} labelled icon-only buttons`);
    for (const file of ["TaskTable.tsx", "TaskDetailsDialog.tsx", "RecoveryPanel.tsx"]) {
      const body = await readFile(`src/components/${file}`, "utf8");
      assert.match(body, /role="region"/, `${file} scrollable table region`);
      assert.match(body, /aria-label=/, `${file} region label`);
      assert.match(body, /<caption/, `${file} table caption`);
    }
  });
  test("transfer progress is polite and never claims completion at 100%", async () => {
    const app = await readFile("src/App.tsx", "utf8");
    assert.match(app, /aria-live="polite"[^>]*>\{progressAnnouncement\}/, "progress has a polite live region");
    assert.match(app, /Math\.floor\(percent \/ 25\)/, "progress is announced per 25% bucket");
    assert.match(app, /bucket >= 4/, "the 100% bucket is not announced as completion");
    assert.match(app, /isTerminal\(event\.status\)\)\s*progressBucket\.current\.delete/, "a restarted task can announce again");
    const feedback = await readFile("src/components/Feedback.tsx", "utf8");
    assert.match(feedback, /role="alert"/, "errors are announced assertively");
  });

  test("stale URL validation error is cleared when the input becomes valid", async () => {
    const dialog = await readFile("src/components/AddDownloadDialog.tsx", "utf8");
    assert.match(
      dialog,
      /if \(validation && validateDownloadUrl\(value\) === null\) setValidation\(null\)/,
      "URL onChange clears an obsolete validation message",
    );
  });

  test("diagnostics preview is visible and redacts sensitive fields", async () => {
    const { redactDiagnosticsBundle } = await server.ssrLoadModule("/src/diagnosticsBundle.ts");
    const bundle = redactDiagnosticsBundle({
      version: "0.1.0",
      database_path: "C:/Users/owner/AppData/Roaming/swiftfetch/state.sqlite",
      headers: [{ name: "Cookie", value: "session=secret" }],
      events: [{ event_type: "download_completed", created_at: "2026-01-01" }],
    });
    assert.match(bundle, /"version": "0.1.0"/);
    assert.doesNotMatch(bundle, /session=secret|AppData\/Roaming/);
    const view = await readFile("src/views/DiagnosticsView.tsx", "utf8");
    assert.match(view, /DiagnosticsBundleDialog/, "preview opens a visible dialog");
  });

  test("multiple browser handoffs are processed one at a time without loss", async () => {
    const { selectPendingId } = await server.ssrLoadModule("/src/pending.ts");
    const rows = Array.from({ length: 8 }, (_, index) => ({ id: `pending-${index}` }));
    assert.equal(selectPendingId(rows, null, false), "pending-0");
    assert.equal(selectPendingId(rows, "pending-0", true), "pending-0", "current dialog stays open");
    assert.equal(selectPendingId(rows.slice(1), null, true), null, "no dialog opens over an open dialog");
    assert.equal(selectPendingId(rows.slice(1), null, false), "pending-1", "next request is not dropped");
    assert.equal(selectPendingId([], null, false), null);
    assert.equal(selectPendingId(rows, "resolved-id", false), "pending-0", "stale id falls back to first row");
  });

  for (const entry of tests) {
    try { await entry.run(); console.log(`PASS ${entry.name}`); }
    catch (error) { failed += 1; console.error(`FAIL ${entry.name}\n${error.stack ?? error}`); }
  }
  console.log(`\nFrontend regression checks: ${tests.length - failed} passed, ${failed} failed, ${tests.length} total.`);
} finally {
  await server.close();
}
process.exitCode = failed ? 1 : 0;
