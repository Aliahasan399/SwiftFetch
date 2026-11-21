import { spawn } from 'node:child_process';
import { mkdtemp, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { createServer } from 'node:net';
const work = await mkdtemp(path.resolve('artifacts/browser-acceptance/diag-'));
await mkdir(path.join(work, 'profile'));
const reservation = createServer();
await new Promise(r => reservation.listen(0, '127.0.0.1', r));
const port = reservation.address().port;
await new Promise(r => reservation.close(r));
const browser = 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const edge = spawn(browser, [`--user-data-dir=${path.join(work,'profile')}`, `--remote-debugging-port=${port}`,
  '--remote-debugging-address=127.0.0.1', '--no-first-run', '--no-default-browser-check',
  '--enable-unsafe-extension-debugging', 'about:blank'], { stdio: 'ignore', windowsHide: false });
try {
  let version;
  for (let i = 0; i < 40; i++) {
    try { version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json(); break; } catch {}
    await delay(250);
  }
  console.log('version', version?.Browser);
  const ws = new WebSocket(version.webSocketDebuggerUrl);
  await new Promise((res, rej) => { ws.addEventListener('open', res, {once:true}); ws.addEventListener('error', rej, {once:true}); });
  const pending = new Map(); let id = 0;
  ws.addEventListener('message', ({data}) => { const m = JSON.parse(data); if (m.id) pending.get(m.id)?.(m); else console.log('EVENT', m.method, JSON.stringify(m.params).slice(0,200)); });
  const call = (method, params) => new Promise((resolve) => { const n = ++id; pending.set(n, resolve); ws.send(JSON.stringify({id:n, method, params})); });
  const packed = path.resolve('artifacts/browser-acceptance/run-ebdc3Y/extension/chromium');
  console.log('sending Extensions.loadUnpacked', packed);
  const result = await call('Extensions.loadUnpacked', { path: packed });
  console.log('RESULT', JSON.stringify(result).slice(0, 1000));
} finally { if (edge.exitCode === null) edge.kill(); }

