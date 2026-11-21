// Phase 3 startup measurement. Runs the instrumented release binary in four
// modes: fresh/cold profile, warm (reused) profile, each with and without CDP.
// Captures Rust phase traces (stderr, SWIFTFETCH_STARTUP_TRACE=1) and CDP
// navigation/performance marks. Privacy-safe: no URLs, paths or settings logged.
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

const binary = path.resolve(process.argv[2] || 'src-tauri/target/release/swiftfetch.exe');
const root = await mkdtemp(path.resolve('artifacts/startup-probe/timing-'));
const results = [];
console.log(`Startup timing: ${root}`);

async function freePort() {
  const s = createServer();
  await new Promise(r => s.listen(0, '127.0.0.1', r));
  const p = s.address().port;
  await new Promise(r => s.close(r));
  return p;
}

for (const mode of ['fresh-cdp', 'fresh-nocdp', 'warm-cdp', 'warm-nocdp']) {
  const fresh = mode.startsWith('fresh');
  const useCdp = mode.endsWith('cdp') && !mode.endsWith('nocdp');
  const folder = path.join(root, mode);
  await mkdir(folder, { recursive: true });
  await mkdir(path.join(folder, 'downloads'), { recursive: true });
  const profile = path.join(folder, 'webview');
  const port = await freePort();
  const env = { ...process.env,
    SWIFTFETCH_DB_PATH: path.join(folder, 'state.sqlite'),
    SWIFTFETCH_DATA_DIR: folder,
    SWIFTFETCH_DOWNLOADS_DIR: path.join(folder, 'downloads'),
    WEBVIEW2_USER_DATA_FOLDER: profile,
    SWIFTFETCH_STARTUP_TRACE: '1',
  };
  if (useCdp) env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-address=127.0.0.1 --remote-debugging-port=${port}`;
  const started = Date.now();
  let stderr = '';
  const app = spawn(binary, [], { cwd: folder, windowsHide: true, env,
    stdio: ['ignore', 'ignore', 'pipe'] });
  app.stderr.on('data', chunk => { stderr += chunk.toString(); });
  let renderedMs = null;
  let cdp = null;
  try {
    if (useCdp) {
      while (Date.now() - started < 60_000 && !cdp) {
        try {
          const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(500) })).json();
          cdp = pages.find(p => p.type === 'page' && /^https?:\/\/tauri\.localhost\//.test(p.url))?.webSocketDebuggerUrl || null;
        } catch {}
        if (!cdp) await delay(100);
      }
      if (cdp) {
        const ws = new WebSocket(cdp);
        await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, { once: true }); ws.addEventListener('error', reject, { once: true }); });
        const pending = new Map(); let id = 0;
        ws.addEventListener('message', ({ data }) => { const m = JSON.parse(data); if (m.id) pending.get(m.id)?.(m); });
        const call = (method, params = {}) => new Promise(resolve => { const n = ++id; pending.set(n, resolve); ws.send(JSON.stringify({ id: n, method, params })); });
        while (Date.now() - started < 60_000) {
          const r = await call('Runtime.evaluate', { expression: "Boolean(document.querySelector('.topbar'))", returnByValue: true });
          if (r.result?.result?.value) { renderedMs = Date.now() - started; break; }
          await delay(200);
        }
        ws.close();
      }
    } else {
      // Native UIA detection without CDP (same approach as startup-probe.mjs).
      const script = path.join(folder, 'uia.ps1');
      await writeFile(script, `Add-Type -AssemblyName UIAutomationClient\nAdd-Type -AssemblyName UIAutomationTypes\n$root=[System.Windows.Automation.AutomationElement]::RootElement\n$cond=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty,'Tauri Window')\n$win=$root.FindFirst([System.Windows.Automation.TreeScope]::Children,$cond)\nif($null -eq $win){'NO_WINDOW';exit}\n$n=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,'Add download')\n$e=$win.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$n)\nif($null -ne $e){'RENDERED'}else{'WINDOW_ONLY'}\n`, { encoding: 'ascii' });
      const { execFile } = await import('node:child_process');
      const { promisify } = await import('node:util');
      const exec = promisify(execFile);
      while (Date.now() - started < 60_000) {
        try {
          const { stdout } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-File', script], { windowsHide: true, timeout: 8000 });
          if (stdout.trim() === 'RENDERED') { renderedMs = Date.now() - started; break; }
        } catch {}
        await delay(400);
      }
    }
    const phases = stderr.split('\n').filter(l => l.includes('swiftfetch-startup:')).map(l => l.trim());
    results.push({ mode, fresh, useCdp, rendered_ms: renderedMs, phases });
    console.log(`${mode}: rendered=${renderedMs}ms phases=${phases.length}`);
  } finally {
    if (app.exitCode === null) {
      const { execFile } = await import('node:child_process');
      const { promisify } = await import('node:util');
      await promisify(execFile)('taskkill.exe', ['/PID', String(app.pid), '/T', '/F'], { windowsHide: true }).catch(() => {});
    }
  }
}
await writeFile(path.join(root, 'timing.json'), JSON.stringify(results, null, 2));
console.log(root);
