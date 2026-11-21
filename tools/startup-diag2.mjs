import { spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';
const exec = promisify((await import('node:child_process')).execFile);
const binary = path.resolve('src-tauri/target/release/swiftfetch.exe');
const work = await mkdtemp(path.resolve('artifacts/startup-probe/mirror-'));
const database = path.join(work, 'state.sqlite');
const destination = path.join(work, 'downloads');
await mkdir(destination);
const reservation = createServer();
await new Promise(r => reservation.listen(0, '127.0.0.1', r));
const port = reservation.address().port;
await new Promise(r => reservation.close(r));
const env = {
  ...process.env,
  SWIFTFETCH_DB_PATH: database,
  SWIFTFETCH_DATA_DIR: work,
  SWIFTFETCH_DOWNLOADS_DIR: destination,
};
const profile = path.join(work, 'webview');
const app = spawn(binary, [], { cwd: work, windowsHide: true, stdio: 'ignore', env: {
  ...env,
  WEBVIEW2_USER_DATA_FOLDER: profile,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-address=127.0.0.1 --remote-debugging-port=${port}`,
}});
const script = path.join(work, 'uia.ps1');
await writeFile(script, `Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root=[System.Windows.Automation.AutomationElement]::RootElement
$cond=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty,'Tauri Window')
$win=$root.FindFirst([System.Windows.Automation.TreeScope]::Children,$cond)
if($null -eq $win){'NO_WINDOW';exit}
$n=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,'Add download')
$e=$win.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$n)
if($null -ne $e){'RENDERED'}else{'WINDOW_ONLY'}
`, { encoding: 'ascii' });
try {
  for (let i = 0; i < 30; i++) {
    await delay(500);
    try {
      const { stdout } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-File', script], { windowsHide: true, timeout: 8000 });
      console.log(i * 500, stdout.trim());
      if (stdout.trim() === 'RENDERED') break;
    } catch (e) { console.log(i * 500, 'ERR', String(e).slice(0, 80)); }
  }
  console.log(work);
} finally { if (app.exitCode === null) app.kill(); }
