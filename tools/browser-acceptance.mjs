import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:http';
import { createServer as tcpServer } from 'node:net';
import { randomUUID } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as delay } from 'node:timers/promises';

const exec = promisify(execFile);
const root = process.cwd();
const binary = path.resolve(process.argv[2] || 'src-tauri/target/release/swiftfetch.exe');
const browser = process.env.SWIFTFETCH_TEST_BROWSER
  || process.env.SWIFTFETCH_TEST_EDGE
  || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const browserLabel = process.env.SWIFTFETCH_TEST_BROWSER ? 'Google Chrome' : 'Microsoft Edge';
const artifacts = path.join(root, 'artifacts/browser-acceptance');
await mkdir(artifacts, { recursive: true });
const workspace = await mkdtemp(path.join(artifacts, 'run-'));
const environment = { ...process.env, SWIFTFETCH_DB_PATH: path.join(workspace, 'state.sqlite'), SWIFTFETCH_DATA_DIR: workspace,
  SWIFTFETCH_DOWNLOADS_DIR: path.join(workspace, 'downloads'), WEBVIEW2_USER_DATA_FOLDER: path.join(workspace, 'webview') };
await mkdir(environment.SWIFTFETCH_DOWNLOADS_DIR);
const packaged = path.join(workspace, 'extension');
await exec(process.execPath, ['browser-extension/tools/package.mjs', packaged], { cwd: root });
const fixtureBytes = Buffer.from('SwiftFetch actual Edge Native Messaging acceptance\n'.repeat(1024));
const fixture = createServer((request, response) => {
  const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range || '');
  const start = range ? Number(range[1]) : 0;
  const end = range?.[2] ? Number(range[2]) : fixtureBytes.length - 1;
  const headers = { 'Content-Type':'application/octet-stream', 'Content-Length':end - start + 1, 'Accept-Ranges':'bytes', ETag:'"browser-acceptance-v1"' };
  if (range) headers['Content-Range'] = `bytes ${start}-${end}/${fixtureBytes.length}`;
  response.writeHead(range ? 206 : 200, headers).end(fixtureBytes.subarray(start, end + 1));
});
const listen = (server) => new Promise((resolve, reject) => {
  server.once('error', reject); server.listen(0, '127.0.0.1', () => resolve(server.address().port));
});
async function until(description, operation, timeout=30_000) {
  const deadline=Date.now()+timeout;
  while(Date.now()<deadline){const result=await operation();if(result)return result;await delay(100);}
  throw new Error(`Timed out: ${description}`);
}
let desktop, edge, socket, browserSocket, registered=false;
const manifest = path.join(workspace, 'host/com.swiftfetch.app.json');
const browserVendor = process.env.SWIFTFETCH_TEST_BROWSER ? 'Chrome' : 'Edge';
const registration = async (action, id) => exec('powershell.exe', ['-NoProfile','-NonInteractive','-File','tools/test-browser-registration.ps1',
  '-Action',action,'-ManifestPath',manifest, '-Browser',browserVendor, ...(id?['-ExecutablePath',binary,'-ExtensionId',id]:[])], { cwd:root });
try {
  const fixturePort=await listen(fixture);
  desktop=spawn(binary,[],{cwd:workspace,env:environment,stdio:'ignore',windowsHide:true});
  await until('desktop queue',async()=>{try{return(await stat(environment.SWIFTFETCH_DB_PATH)).size>0}catch{return false}});
  await until('desktop defaults',async()=>{
    const db=new DatabaseSync(environment.SWIFTFETCH_DB_PATH,{timeout:3000});
    try {
      if(!db.prepare("SELECT COUNT(*) AS n FROM sqlite_master WHERE name='settings'").get().n)return false;
      const row=db.prepare("SELECT value_json FROM settings WHERE key='app_settings'").get();
      const settings=row?JSON.parse(row.value_json):{
        destination_root:environment.SWIFTFETCH_DOWNLOADS_DIR,max_active_tasks:3,connections_per_task:4,
        global_speed_limit:0,default_task_speed_limit:0,max_retries:3,overwrite_policy:'rename',verify_checksums:true,
        scan_with_antivirus:false,scanner_command:'',prevent_sleep:false,start_on_login:false,theme:'dark',retention_days:90,
        browser_integration:false,proxy:'',scheduler_enabled:true,shutdown_after_complete:false,keep_partial_on_failure:true,
      };
      db.prepare("INSERT OR REPLACE INTO settings(key,value_json,updated_at)VALUES('app_settings',?,?)")
        .run(JSON.stringify({...settings,destination_root:environment.SWIFTFETCH_DOWNLOADS_DIR,browser_integration:true}),new Date().toISOString());
      return true;
    }catch(error){if(error.errcode===5)return false;throw error;}finally{db.close();}
  });
  const reservation=tcpServer();const port=await listen(reservation);await new Promise(resolve=>reservation.close(resolve));
  edge=spawn(browser,[`--user-data-dir=${path.join(workspace,'edge-profile')}`,`--remote-debugging-port=${port}`,
    '--remote-debugging-address=127.0.0.1','--no-first-run','--no-default-browser-check','--enable-unsafe-extension-debugging',
    ...(process.env.SWIFTFETCH_TEST_HEADFUL ? [] : ['--headless=new']),
    'about:blank'],
    {env:environment,stdio:'ignore',windowsHide:false});
  const version=await until('Edge remote debugging',async()=>{
    try{const response=await fetch(`http://127.0.0.1:${port}/json/version`,{signal:AbortSignal.timeout(1000)});return await response.json();}catch{return null}
  });
  browserSocket=new WebSocket(version.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{browserSocket.addEventListener('open',resolve,{once:true});browserSocket.addEventListener('error',reject,{once:true});});
  const loaded=new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Edge extension load timed out')),90_000);
    browserSocket.addEventListener('message',({data})=>{const result=JSON.parse(data);if(result.id===10){clearTimeout(timer);resolve(result);}else if(process.env.SWIFTFETCH_DEBUG){console.log('CDP',result.id,result.method||JSON.stringify(result.error||result.result).slice(0,120));}});
  });
  browserSocket.send(JSON.stringify({id:10,method:'Extensions.loadUnpacked',params:{path:path.join(packaged,'chromium')}}));
  const loadedResult=await loaded;
  await writeFile(path.join(workspace,'browser-load.json'),JSON.stringify({version:version.Browser,result:loadedResult},null,2));
  if(loadedResult.error)throw new Error(`Edge refused the unpacked extension: ${loadedResult.error.message}`);
  const installedId=loadedResult.result.id;
  assert.match(installedId,/^[a-p]{32}$/);
  const worker=await until('the installed Edge extension worker',async()=>{
    try{const response=await fetch(`http://127.0.0.1:${port}/json/list`,{signal:AbortSignal.timeout(1000)});
      return(await response.json()).find(target=>target.type==='service_worker'&&target.url.startsWith(`chrome-extension://${installedId}/`));
    }catch{return null}
  });
  const extensionId=new URL(worker.url).host;assert.match(extensionId,/^[a-p]{32}$/);
  await registration('Register',extensionId);registered=true;
  socket=new WebSocket(worker.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  const intent={protocol:1,type:'download_intent',request_id:randomUUID(),url:`http://127.0.0.1:${fixturePort}/browser.bin`,
    suggested_name:'browser-acceptance.bin',page_origin:`http://127.0.0.1:${fixturePort}`,content_type:'application/octet-stream',user_initiated:true,cookies:[],expires_at:new Date(Date.now()+120_000).toISOString()};
  const result=new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Native Messaging response timed out')),12_000);
    socket.addEventListener('message',({data})=>{const value=JSON.parse(data);if(value.id===1){clearTimeout(timer);resolve(value);}});
  });
  socket.send(JSON.stringify({id:1,method:'Runtime.evaluate',params:{expression:`chrome.runtime.sendNativeMessage('com.swiftfetch.app',${JSON.stringify(intent)})`,awaitPromise:true,returnByValue:true}}));
  const reply=await result;
  await writeFile(path.join(workspace,'native-reply.json'),JSON.stringify(reply,null,2));
  assert.deepEqual(reply.result?.result?.value,{ok:true,value:{task_id:intent.request_id}});
  await until('the browser-originated file',async()=>{try{return(await readFile(path.join(environment.SWIFTFETCH_DOWNLOADS_DIR,'browser-acceptance.bin'))).equals(fixtureBytes)}catch{return false}});
  await writeFile(path.join(workspace,'result.json'),JSON.stringify({browser:browserLabel,native_messaging:true,isolated_profile:true,temporary_registration:true,passed:true},null,2));
  console.log(`PASS actual ${browserLabel} extension load, native host spawn, queue intake and output: ${workspace}`);
}finally{
  socket?.close();
  browserSocket?.close();
  if(edge?.pid){await exec('taskkill.exe',['/PID',String(edge.pid),'/T','/F']).catch(()=>{});}
  desktop?.kill();
  try { if(registered)await registration('Unregister'); }
  finally { fixture.closeAllConnections();fixture.close(); }
}
