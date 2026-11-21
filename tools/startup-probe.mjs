// Repeated cold-start probe using native UI Automation.
// Attaching CDP is intentionally avoided for assertions; the reserved debug
// port mirrors the accepted smoke harness. UIA verifies React rendered.
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

const executable = path.resolve(process.argv[2] || 'src-tauri/target/release/swiftfetch.exe');
const count = Number(process.argv[3] || 12);
const exec = promisify(execFile);
await mkdir('artifacts/startup-probe', { recursive: true });
const root = await mkdtemp(path.resolve('artifacts/startup-probe/run-'));
const results = [];
console.log(`Startup probe (native UI): ${root}`);

const SCRIPT = [
  'Add-Type -AssemblyName UIAutomationClient',
  'Add-Type -AssemblyName UIAutomationTypes',
  '$root=[System.Windows.Automation.AutomationElement]::RootElement',
  "$cond=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty,'Tauri Window')",
  '$win=$root.FindFirst([System.Windows.Automation.TreeScope]::Children,$cond)',
  "if($null -eq $win){'NO_WINDOW';exit}",
  "$n=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,'Add download')",
  '$e=$win.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$n)',
  "if($null -ne $e){'RENDERED'}else{'WINDOW_ONLY'}",
].join('\n');
const script = path.join(root, 'uia.ps1');
await writeFile(script, SCRIPT + '\n', { encoding: 'ascii' });

for (let attempt = 0; attempt < count; attempt++) {
  const folder = path.join(root, String(attempt + 1));
  await mkdir(folder);
  await mkdir(path.join(folder, 'downloads'));
  const reservation = createServer();
  await new Promise((resolve) => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise((resolve) => reservation.close(resolve));
  const started = Date.now();
  const app = spawn(executable, [], {
    cwd: folder,
    windowsHide: true,
    stdio: 'ignore',
    env: {
      ...process.env,
      SWIFTFETCH_DB_PATH: path.join(folder, 'state.sqlite'),
      SWIFTFETCH_DATA_DIR: folder,
      SWIFTFETCH_DOWNLOADS_DIR: path.join(folder, 'downloads'),
      WEBVIEW2_USER_DATA_FOLDER: path.join(folder, 'webview'),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-address=127.0.0.1 --remote-debugging-port=${port}`,
    },
  });
  let state = 'NO_WINDOW';
  try {
    while (Date.now() - started < 60_000) {
      if (app.exitCode !== null) throw new Error(`App exited ${app.exitCode}`);
      try {
        const { stdout } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-File', script], { windowsHide: true, timeout: 8_000 });
        state = stdout.trim();
      } catch { state = 'NO_WINDOW'; }
      if (state === 'RENDERED') break;
      await delay(400);
    }
    const rendered = state === 'RENDERED';
    results.push({ attempt: attempt + 1, rendered, state, millis: Date.now() - started });
    console.log(`${rendered ? 'PASS' : 'FAIL'} startup ${attempt + 1}: ${state} in ${Date.now() - started} ms`);
    if (!rendered) {
      await writeFile(path.join(folder, 'failure.json'), JSON.stringify({ state, millis: Date.now() - started }, null, 2));
      break;
    }
  } finally {
    if (app.exitCode === null) {
      await exec('taskkill.exe', ['/PID', String(app.pid), '/T', '/F'], { windowsHide: true }).catch(() => {});
    }
    await delay(500);
  }
}
await writeFile(path.join(root, 'results.json'), JSON.stringify(results, null, 2));
if (results.length !== count || results.some((result) => !result.rendered)) process.exitCode = 1;
