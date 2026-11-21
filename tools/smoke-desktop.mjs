// Exercise the packaged Windows app with an isolated database and WebView2 profile.
import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { access, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { createServer as createHttpServer } from "node:http";
import { createServer as createTcpServer } from "node:net";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import { setTimeout as delay } from "node:timers/promises";

const localhost = "127.0.0.1";
const executable = path.resolve(process.argv.slice(2).find((argument) => !argument.startsWith("--")) ?? "src-tauri/target/release/swiftfetch.exe");
const outputRoot = path.resolve("artifacts/smoke");
const body = Buffer.alloc(4 * 1024 * 1024);
for (let index = 0; index < body.length; index++) {
  body[index] = (index * 17 + (index >>> 12)) & 255;
}
const digest = createHash("sha256").update(body).digest("hex");
const runStress = process.argv.includes("--stress");
const nativeExec = promisify(execFile);

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, localhost, () => resolve(server.address().port));
  });
}

async function waitFor(description, check, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const result = await check();
    if (result) return result;
    await delay(100);
  }
  throw new Error(`Timed out waiting for ${description}`);
}

class DevTools {
  nextId = 0;
  pending = new Map();

  constructor(url) {
    this.socket = new WebSocket(url);
    this.ready = new Promise((resolve, reject) => {
      this.socket.addEventListener("open", resolve, { once: true });
      this.socket.addEventListener("error", () => reject(new Error("WebView2 debugging connection failed")), { once: true });
    });
    this.socket.addEventListener("message", ({ data }) => {
      const message = JSON.parse(data);
      this.pending.get(message.id)?.(message);
    });
  }

  async command(method,params={}) {
    await this.ready;
    const id = ++this.nextId;
    const result = await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error("WebView2 evaluation timed out"));
      }, 8_000);
      this.pending.set(id, (message) => {
        clearTimeout(timeout);
        this.pending.delete(id);
        if (message.error) reject(new Error(message.error.message));
        else resolve(message.result);
      });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
    return result;
  }

  async evaluate(expression) {
    const result=await this.command("Runtime.evaluate",{expression,returnByValue:true,awaitPromise:true});
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    return result.result.value;
  }

  async invoke(command, args = {}) {
    const result = await this.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})`);
    assert.ok(result?.ok, `${command} failed (${result?.error?.code ?? "invalid envelope"})`);
    return result.data;
  }

  close() {
    this.socket.close();
  }
}

async function main() {
  assert.equal(process.platform, "win32", "This check requires Windows and WebView2");
  await access(executable);
  await mkdir(outputRoot, { recursive: true });
  const work = await mkdtemp(path.join(outputRoot, "run-"));
  const destination = path.join(work, "downloads");
  const database = path.join(work, "state.sqlite");
  const appEnvironment = {
    ...process.env,
    SWIFTFETCH_DB_PATH: database,
    SWIFTFETCH_DATA_DIR: work,
    SWIFTFETCH_DOWNLOADS_DIR: destination,
  };
  await mkdir(destination);
  console.log(`Desktop smoke data: ${work}`);

  // Phase 1 evidence: capture test-group outcomes without recording URLs,
  // secrets, clipboard text, or other sensitive values.
  const groupResults = [];
  const originalLog = console.log;
  console.log = (...args) => {
    const line = args.map((value) => String(value)).join(' ');
    if (/^(PASS|SKIP) /.test(line)) groupResults.push(line);
    originalLog(...args);
  };
  let runError = null;
  const version = JSON.parse(await readFile('package.json', 'utf8')).version;

  let holdTransfer = true;
  let holdAdmission = true;
  const recoveryStarts = [];
  const fixture = createHttpServer((request, response) => {
    const resource = new URL(request.url, `http://${localhost}`).pathname;
    const resourceSize = resource === "/large.bin" ? 256 * 1024 * 1024 : body.length;
    if (!["/ready.bin", "/recovery.bin", "/browser.bin", "/admission.bin", "/large.bin"].includes(resource)) {
      response.writeHead(404).end();
      return;
    }
    const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? "");
    const start = range ? Number(range[1]) : 0;
    const end = range?.[2] ? Number(range[2]) : resourceSize - 1;
    if (resource === "/recovery.bin") recoveryStarts.push(start);
    if (start > end || end >= resourceSize) {
      response.writeHead(416, { "Content-Range": `bytes */${resourceSize}` }).end();
      return;
    }
    const headers = {
      "Content-Length": end - start + 1,
      "Content-Type": "application/octet-stream",
      "Accept-Ranges": "bytes",
      ETag: '"swiftfetch-desktop-fixture-1"',
    };
    if (range) headers["Content-Range"] = `bytes ${start}-${end}/${resourceSize}`;
    response.writeHead(range ? 206 : 200, headers);
    if (request.method === "HEAD") response.end();
    else if (resource === "/large.bin") {
      let offset = start;
      const stream = () => {
        while (offset <= end && !response.destroyed) {
          const index = offset % body.length;
          const length = Math.min(64 * 1024, body.length - index, end - offset + 1);
          const bytes = body.subarray(index, index + length);
          offset += length;
          if (!response.write(bytes)) { response.once("drain", stream); return; }
        }
        response.end();
      };
      stream();
    }
    else if (((resource === "/recovery.bin" && holdTransfer) || (resource === "/admission.bin" && holdAdmission)) && end - start > 2 * 1024 * 1024) {
      response.write(body.subarray(start, start + 2 * 1024 * 1024));
    } else response.end(body.subarray(start, end + 1));
  });

  let app;
  let devtools;
  let launches = 0;
  async function stopApp() {
    devtools?.close();
    devtools = undefined;
    if (app && app.exitCode === null && app.signalCode === null) {
      app.kill();
      await waitFor("the isolated app process to exit", () => app.exitCode !== null || app.signalCode !== null, 5_000);
    }
  }

  async function startApp(expected = "desktop") {
    const reservation = createTcpServer();
    const port = await listen(reservation);
    await new Promise((resolve) => reservation.close(resolve));
    const profile = path.join(work, `webview-${++launches}`);
    app = spawn(executable, [], {
      cwd: work,
      windowsHide: true,
      stdio: "ignore",
      env: {
        ...appEnvironment,
        WEBVIEW2_USER_DATA_FOLDER: profile,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-address=${localhost} --remote-debugging-port=${port}`,
      },
    });
    let spawnError;
    app.once("error", (error) => { spawnError = error; });
    const endpoint = await waitFor("the packaged SwiftFetch WebView2 page", async () => {
      if (spawnError) throw spawnError;
      if (app.exitCode !== null) throw new Error(`SwiftFetch exited during launch (${app.exitCode})`);
      let pages;
      try {
        const response = await fetch(`http://${localhost}:${port}/json/list`, { signal: AbortSignal.timeout(1_000) });
        pages = await response.json();
      } catch {
        return null;
      }
      return pages.find((page) => page.type === "page" && /^https?:\/\/tauri\.localhost\//.test(page.url))?.webSocketDebuggerUrl;
    });
    const url = new URL(endpoint);
    assert.equal(url.protocol, "ws:");
    assert.equal(url.hostname, localhost);
    assert.equal(url.port, String(port));
    devtools = new DevTools(endpoint);
    try {
      await waitFor("the application UI", () => devtools.evaluate(expected === "recovery"
        ? "document.querySelector('h1')?.textContent === 'SwiftFetch startup recovery'"
        : "Boolean(document.querySelector('.topbar'))"));
    } catch (error) {
      const details = await devtools.evaluate("({ title: document.title, ready: document.readyState, scripts: Array.from(document.scripts, (script) => script.getAttribute('src')), body: document.body?.textContent.slice(0, 120) })").catch(() => null);
      const screenshot=await devtools.command('Page.captureScreenshot',{format:'png'}).catch(()=>null);
      if(screenshot?.data)await writeFile(path.join(work,`startup-${launches}.png`),Buffer.from(screenshot.data,'base64'));
      const frames=await devtools.command('Page.getFrameTree').catch(()=>null);
      await writeFile(path.join(work,`startup-${launches}.json`),JSON.stringify({details,frames},null,2));
      throw new Error(`Desktop launch ${launches} did not render: ${JSON.stringify(details)}`, { cause: error });
    }
    if (expected === "desktop") await devtools.invoke("get_settings");
  }

  async function sendNativeIntent(intent, args) {
    const payload = Buffer.from(JSON.stringify(intent));
    const header = Buffer.alloc(4);
    header.writeUInt32LE(payload.length);
    const host = spawn(executable, args, {
      cwd: work,
      env: appEnvironment,
      windowsHide: true,
      stdio: ["pipe", "pipe", "ignore"],
      signal: AbortSignal.timeout(12_000),
    });
    const chunks = [];
    host.stdout.on("data", (chunk) => chunks.push(chunk));
    const result = new Promise((resolve, reject) => {
      host.once("error", reject);
      host.once("close", (code) => code === 0 ? resolve(Buffer.concat(chunks)) : reject(new Error(`Native host exited with code ${code}`)));
    });
    host.stdin.end(Buffer.concat([header, payload]));
    const response = await result;
    assert.ok(response.length >= 4, "no framed Native Messaging reply received");
    assert.equal(response.readUInt32LE(), response.length - 4);
    return JSON.parse(response.subarray(4));
  }

  async function waitForTask(taskId, status) {
    return waitFor(`task status ${status}`, async () => {
      const task = await devtools.invoke("get_task", { taskId });
      if (task.status === "failed") throw new Error(`Transfer failed (${task.error_code})`);
      return task.status === status && task;
    });
  }

  async function verifyFile(name) {
    const bytes = await readFile(path.join(destination, name));
    assert.equal(createHash("sha256").update(bytes).digest("hex"), digest);
  }

  try {
    const port = await listen(fixture);
    const base = `http://${localhost}:${port}`;
    await startApp();
    console.log("PASS packaged UI and get_settings over Tauri IPC");

    await devtools.evaluate("document.querySelector('.topbar-actions .button-primary').click()");
    await waitFor("the add-download form", () => devtools.evaluate("Boolean(document.querySelector('dialog[open] form'))"));
    await devtools.evaluate(`(() => {
      const dialog = document.querySelector('dialog[open]');
      const setValue = (labelText, value) => {
        const label = Array.from(dialog.querySelectorAll('label')).find((item) => item.textContent.trim().startsWith(labelText));
        const input = label.querySelector('input');
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
        input.dispatchEvent(new Event('input', { bubbles: true }));
      };
      setValue('Download URL', ${JSON.stringify(`${base}/ready.bin`)});
      setValue('Destination folder', ${JSON.stringify(destination)});
    })()`);
    const submit = "document.querySelector('dialog[open] button[type=submit]')";
    await waitFor("the add-download submit button", () => devtools.evaluate(`!${submit}.disabled`));
    await devtools.evaluate(`${submit}.click()`);
    const ready = await waitFor("the download added by the UI", async () => {
      const result = await devtools.invoke("list_tasks", { limit: 10 });
      return result.tasks.find((task) => task.display_name === "ready.bin");
    });
    const finished = await waitForTask(ready.id, "completed");
    assert.ok(finished.segment_count > 1, "the fixture should be downloaded in multiple segments");
    await devtools.evaluate("Array.from(document.querySelectorAll('nav button')).find((button) => button.querySelector('span')?.textContent === 'History').click()");
    await waitFor("the UI to render the finished file", () => devtools.evaluate("Array.from(document.querySelectorAll('button')).some((button) => button.textContent.trim() === 'ready.bin')"));
    await verifyFile("ready.bin");
    console.log("PASS Add download in the UI, task read-back, and segmented output checksum");

    const recovery = await devtools.invoke("add_download", { req: {
      url: `${base}/recovery.bin`, suggested_name: "recovery.bin", destination_directory: destination,
      connections: 1, headers: [], checksum: { algorithm: "sha256", value: digest },
    } });
    await waitFor("an interrupted transfer with bytes on disk", async () => {
      const task = await devtools.invoke("get_task", { taskId: recovery.id });
      if (task.status === "failed") throw new Error(`Transfer failed (${task.error_code})`);
      if (!["starting", "downloading"].includes(task.status) || !task.temporary_path) return false;
      const partial = await stat(task.temporary_path).catch((error) => {
        if (error.code === "ENOENT") return null;
        throw error;
      });
      if (partial?.size < 1024 * 1024) return false;
      const snapshot = new DatabaseSync(database, { readOnly: true });
      try {
        return snapshot.prepare("SELECT COALESCE(SUM(received_bytes), 0) AS bytes FROM download_segments WHERE download_id = ?")
          .get(recovery.id).bytes >= 1024 * 1024;
      } finally {
        snapshot.close();
      }
    });
    await stopApp();
    holdTransfer = false;
    await startApp();
    await waitForTask(recovery.id, "paused");
    await devtools.invoke("control_download", { req: { task_id: recovery.id, action: "resume", delete_file: false } });
    const completed = await waitForTask(recovery.id, "completed");
    assert.equal(completed.checksum_verified, true);
    await verifyFile("recovery.bin");
    assert.ok(recoveryStarts.some((start) => start >= 1024 * 1024), "restart must reuse a durable checkpoint rather than download from zero");
    console.log("PASS process termination, queue recovery, resume, and checksum verification");

    const browserIntent = {
      protocol: 1, type: "download_intent", request_id: randomUUID(),
      url: `${base}/browser.bin`, suggested_name: "browser.bin", page_origin: base,
      content_type: "application/octet-stream", user_initiated: true, cookies: [],
      expires_at: new Date(Date.now() + 120_000).toISOString(),
    };
    const chromium = ["chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/", "--parent-window=0"];
    const firefox = [path.join(work, "com.swiftfetch.app.json"), "swiftfetch@localhost"];
    const settings = await devtools.invoke("get_settings");
    assert.equal(settings.browser_integration, false);
    assert.equal((await sendNativeIntent(browserIntent, chromium)).code, "bridge_disabled");
    // Opt in only in this isolated database. Saving typical settings through
    // the GUI can also update this Windows user's start-on-login preference.
    const db = new DatabaseSync(database, { timeout: 3_000 });
    try {
      db.prepare("INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES ('app_settings', ?, ?)")
        .run(JSON.stringify({ ...settings, browser_integration: true }), new Date().toISOString());
    } finally {
      db.close();
    }
    const receipt = await sendNativeIntent(browserIntent, chromium);
    assert.deepEqual(receipt, { ok: true, value: { task_id: browserIntent.request_id } });
    assert.equal((await sendNativeIntent(browserIntent, firefox)).code, "replayed_intent");
    assert.equal((await sendNativeIntent({ ...browserIntent, cookies: [{}] }, chromium)).code, "invalid_intent");
    await waitForTask(browserIntent.request_id, "completed");
    await verifyFile("browser.bin");
    console.log("PASS native executable handoff, opt-in, replay guard, credential rejection, and final file");

    const current = await devtools.invoke("get_settings");
    await devtools.invoke("save_settings", { settings: { ...current, max_active_tasks: 1 } });
    const held = await devtools.invoke("add_download", { req: {
      url: `${base}/admission.bin`, suggested_name: "admission.bin", destination_directory: destination,
      connections: 1, headers: [],
    } });
    await waitForTask(held.id, "downloading");
    const waiting = await devtools.invoke("add_download", { req: {
      url: `${base}/ready.bin`, suggested_name: "waiting.bin", destination_directory: destination,
      connections: 1, headers: [],
    } });
    const queue = await devtools.invoke("get_task", { taskId: waiting.id });
    assert.equal(queue.status, "queued");
    await devtools.invoke("control_download", { req: { task_id: held.id, action: "pause" } });
    await waitForTask(waiting.id, "completed");
    await verifyFile("waiting.bin");
    holdAdmission = false;
    await devtools.invoke("control_download", { req: { task_id: held.id, action: "resume" } });
    await waitForTask(held.id, "completed");
    await verifyFile("admission.bin");
    console.log("PASS queue admission limit, pause releases its slot, and queued resume completes");

    const directSettings = await devtools.invoke("get_settings");
    await devtools.invoke("save_settings", { settings: { ...directSettings, proxy: base } });
    const proxied = await devtools.invoke("add_download", { req: {
      url: "http://swiftfetch.invalid/ready.bin", suggested_name: "proxy.bin", destination_directory: destination,
      connections: 1, headers: [],
    } });
    await waitForTask(proxied.id, "completed");
    await verifyFile("proxy.bin");
    await devtools.invoke("save_settings", { settings: directSettings });
    console.log("PASS configured HTTP proxy handles a target without external DNS or network requests");

    const updates = await devtools.invoke("get_update_state");
    assert.equal(updates.current_version, "0.1.0");
    assert.ok(typeof updates.configured === "boolean");
    if (!updates.configured) assert.equal((await devtools.invoke("check_for_updates")).phase, "not_configured");
    await devtools.invoke("report_ready");
    console.log("PASS updater reports publisher configuration and accepts the desktop health handshake");

    const segments = await devtools.invoke("get_task_segments", {taskId:ready.id});
    assert.ok(segments.length > 1 && segments.every((segment) => segment.status === "completed"));
    const backup = await devtools.invoke("backup_metadata");
    assert.ok((await stat(backup)).size > 0);
    const inventory = await devtools.invoke("export_queue");
    assert.ok(JSON.parse(inventory).tasks.length >= 1);
    assert.ok(!inventory.includes('headers_json'));
    console.log("PASS task segment inspection, consistent local backup, and URL-stripped inventory export");

    const beforeCollision = await devtools.invoke("get_settings");
    await devtools.invoke("save_settings", {settings:{...beforeCollision,overwrite_policy:"ask"}});
    const collision = await devtools.invoke("add_download", {req:{
      url:`${base}/ready.bin`,suggested_name:"ready.bin",destination_directory:destination,headers:[],
    }});
    await waitFor('the explicit collision decision',async()=>{
      const row=await devtools.invoke('get_task',{taskId:collision.id});
      return row.status==='failed' && row.error_code==='OVERWRITE_CONFIRMATION_REQUIRED';
    });
    await devtools.invoke('control_download',{req:{task_id:collision.id,action:'rename'}});
    const resolved=await waitForTask(collision.id,'completed');
    assert.ok(resolved.destination_path.endsWith('ready (1).bin'));
    await verifyFile('ready (1).bin');await verifyFile('ready.bin');
    await devtools.invoke("save_settings", {settings:beforeCollision});
    console.log('PASS Ask collision retains output and explicit Rename publishes without another transfer');

    await devtools.evaluate(`(() => {
      const data=new DataTransfer();data.setData('text/uri-list',${JSON.stringify(`${base}/ready.bin`)});
      window.dispatchEvent(new DragEvent('drop',{dataTransfer:data,bubbles:true,cancelable:true}));
    })()`);
    await waitFor('the dropped URL confirmation form',()=>devtools.evaluate(`document.querySelector('dialog[open] input[inputmode=url]')?.value===${JSON.stringify(`${base}/ready.bin`)}`));
    await devtools.evaluate(`(() => {
      const data=new DataTransfer();data.setData('text/uri-list',${JSON.stringify(`${base}/browser.bin`)});
      window.dispatchEvent(new DragEvent('drop',{dataTransfer:data,bubbles:true,cancelable:true}));
    })()`);
    assert.equal(await devtools.evaluate("document.querySelector('dialog[open] input[inputmode=url]')?.value"),`${base}/ready.bin`,"a second intake event must preserve the open form");
    await devtools.evaluate("document.querySelector('dialog[open] button[aria-label=\"Close dialog\"]').click()");
    console.log('PASS URL drop opens confirmation and a second intake preserves unsaved dialog input');

    const clipboardCommand='[Console]::InputEncoding=[Text.Encoding]::UTF8; $value=[Console]::In.ReadToEnd(); Set-Clipboard -Value $value';
    const writeClipboard=(text)=>new Promise((resolve,reject)=>{
      const child=spawn('powershell.exe',['-NoProfile','-NonInteractive','-Command',clipboardCommand],{windowsHide:true,stdio:['pipe','ignore','pipe']});
      child.once('error',reject);child.once('close',code=>code===0?resolve():reject(new Error('Clipboard test setup failed')));child.stdin.end(text);
    });
    const clipboardSnapshotCommand="Add-Type -AssemblyName System.Windows.Forms; [Console]::OutputEncoding=[Text.Encoding]::UTF8; $data=[Windows.Forms.Clipboard]::GetDataObject(); $formats=if($data){@($data.GetFormats())}else{@()}; $allowed=@('Text','UnicodeText','OEMText','System.String','Locale'); @{can_restore=(@($formats|Where-Object{$_ -notin $allowed}).Count -eq 0);text=[Windows.Forms.Clipboard]::GetText()}|ConvertTo-Json -Compress";
    const previousClipboard=process.argv.includes('--allow-clipboard-test')
      ? JSON.parse((await nativeExec('powershell.exe',['-NoProfile','-NonInteractive','-STA','-Command',clipboardSnapshotCommand],{windowsHide:true})).stdout)
      : {can_restore:false};
    const clipboardFixture=`${base}/ready.bin`;
    if(previousClipboard.can_restore){
      try {
        await writeClipboard(clipboardFixture);
        assert.equal(await devtools.invoke('read_clipboard_url'),clipboardFixture);
      } finally {
        const now=await devtools.invoke('read_clipboard_url').catch(()=>null);
        if(now===clipboardFixture)await writeClipboard(previousClipboard.text);
      }
      console.log('PASS explicit clipboard URL capture; prior clipboard text restored without logging its contents');
    }else{
      console.log('SKIP OS clipboard mutation by default; use --allow-clipboard-test only in an isolated desktop session');
    }

    const unnamed = await devtools.evaluate(`Array.from(document.querySelectorAll('button,input,select,textarea')).filter(element =>
      element.getClientRects().length && !element.disabled && !element.getAttribute('aria-label') && !element.getAttribute('aria-labelledby') &&
      !(element.tagName==='BUTTON' && element.textContent.trim()) && !element.labels?.length).length`);
    assert.equal(unnamed,0,'visible controls need accessible text or labels');
    console.log('PASS visible desktop controls expose text or accessible labels');

    // Confirm the power request and cancel it promptly. The test never reaches
    // the OS shutdown command, and finally always kills its isolated app.
    const beforePower=await devtools.invoke('get_settings');
    await devtools.invoke('save_settings',{settings:{...beforePower,shutdown_after_complete:true}});
    const powerTask=await devtools.invoke('add_download',{req:{url:`${base}/ready.bin`,suggested_name:'power.bin',destination_directory:destination,headers:[]}});
    await waitForTask(powerTask.id,'completed');
    const pendingPower=await waitFor('the cancellable shutdown request',()=>devtools.invoke('get_power_request'));
    await devtools.invoke('cancel_shutdown',{requestId:pendingPower.request_id});
    assert.equal(await devtools.invoke('get_power_request'),null);
    await devtools.invoke('save_settings',{settings:beforePower});
    console.log('PASS opt-in completion shutdown creates a countdown and cancellation removes it before any OS action');

    if (runStress) {
      const memory = async () => {
        const result = await nativeExec("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", `(Get-Process -Id ${app.pid}).WorkingSet64`], { windowsHide: true });
        return Number(result.stdout.trim());
      };
      const baseline = await memory();
      const large = await devtools.invoke("add_download", { req: {
        url: `${base}/large.bin`, suggested_name: "large.bin", destination_directory: destination,
        connections: 4, speed_limit: 16 * 1024 * 1024, headers: [],
      } });
      let peak = baseline;
      const deadline = Date.now() + 90_000;
      let finished = false;
      while (Date.now() < deadline) {
        peak = Math.max(peak, await memory());
        const row = await devtools.invoke("get_task", { taskId: large.id });
        if (row.status === "failed") throw new Error(`Large transfer failed (${row.error_code})`);
        if (row.status === "completed") { finished = true; break; }
        await delay(500);
      }
      assert.ok(finished, "the large streaming transfer did not complete");
      assert.ok(peak - baseline < 192 * 1024 * 1024, "Rust working-set growth exceeded 192 MiB for a 256 MiB transfer");
      const expected = createHash("sha256");
      for (let block = 0; block < 64; block++) expected.update(body);
      const { createReadStream } = await import("node:fs");
      const actual = createHash("sha256");
      for await (const chunk of createReadStream(path.join(destination, "large.bin"))) actual.update(chunk);
      assert.equal(actual.digest("hex"), expected.digest("hex"));
      console.log(`PASS 256 MiB streaming transfer, checksum, Rust peak growth ${Math.round((peak - baseline) / 1024 / 1024)} MiB`);
    }

    const disk = await devtools.invoke('get_disk_space', {path:destination});
    assert.ok(disk.available_bytes >= 0 && disk.total_bytes > 0 && disk.available_bytes <= disk.total_bytes);
    const backupName = path.basename(backup);
    assert.ok((await devtools.invoke('get_recovery_state')).backups.some(item=>item.name===backupName));
    const unconfirmed = await devtools.evaluate(`window.__TAURI_INTERNALS__.invoke('restore_metadata',{backupName:${JSON.stringify(backupName)},confirm:false})`);
    assert.equal(unconfirmed.error?.code,'CONFIRMATION_REQUIRED');
    const restored = await devtools.invoke('restore_metadata',{backupName,confirm:true});
    assert.equal(restored.restart_required,false);
    assert.ok(restored.restored_tasks>0 && (await stat(restored.safety_backup)).size>0);
    const afterRestore = await devtools.invoke('get_settings');
    assert.equal(afterRestore.shutdown_after_complete,false);
    assert.equal(afterRestore.clipboard_capture,false);
    assert.equal(afterRestore.browser_integration,false);
    assert.equal(afterRestore.scan_with_antivirus,false);
    assert.equal(afterRestore.overwrite_policy,'rename');
    console.log('PASS disk-space IPC, restore confirmation, online SQLite restore, safety snapshot and consent reset');

    await stopApp();
    const corruptBytes = Buffer.from('isolated corrupt database for startup recovery acceptance');
    await writeFile(database,corruptBytes);
    // Remove valid WAL/SHM sidecars so the corruption cannot be replayed away.
    for (const suffix of ['-wal','-shm']) await rm(`${database}${suffix}`,{force:true});
    await startApp('recovery');
    const recoveryState = await devtools.invoke('get_recovery_state');
    assert.ok(recoveryState.message && recoveryState.backups.some(item=>item.name===backupName));
    const offlineRestore = await devtools.invoke('restore_metadata',{backupName,confirm:true});
    assert.equal(offlineRestore.restart_required,true);
    assert.deepEqual(await readFile(path.join(offlineRestore.safety_backup,'state.sqlite')),corruptBytes);
    await stopApp();
    await startApp();
    assert.ok((await devtools.invoke('list_tasks',{limit:100})).tasks.length>0);
    await devtools.invoke('report_ready');
    console.log('PASS corrupt-database startup recovery UI, validated offline restore, preserved corrupt bytes and healthy restart');
  } catch (error) {
    runError = String(error);
    throw error;
  } finally {
    console.log = originalLog;
    try {
      await stopApp();
    } finally {
      fixture.closeAllConnections();
      fixture.close();
      // Phase 1: machine-readable, redacted result for this unique run folder.
      const report = {
        timestamp: new Date().toISOString(),
        app_version: version,
        platform: process.platform,
        binary: path.relative(process.cwd(), executable),
        environment: { isolated_database: true, isolated_webview_profile: true, clipboard_mutation: process.argv.includes('--allow-clipboard-test') },
        groups: groupResults.map((line) => ({ name: line.slice(5), status: line.startsWith('PASS') ? 'pass' : 'skip' })),
        failed: runError !== null,
        error: runError,
        artifact_paths: { database: path.relative(work, database), downloads: path.relative(work, destination), work: path.basename(work) },
        passed_groups: groupResults.filter((line) => line.startsWith('PASS')).length,
        skipped_groups: groupResults.filter((line) => line.startsWith('SKIP')).length,
      };
      await writeFile(path.join(work, 'result.json'), JSON.stringify(report, null, 2));
      originalLog(`Smoke result: ${path.join(work, 'result.json')} (${report.passed_groups} pass, ${report.skipped_groups} skip)`);
    }
  }
}

await main();
