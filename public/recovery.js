'use strict';
const result=document.getElementById('result');
async function command(name,args={}){
  const response=await window.__TAURI_INTERNALS__.invoke(name,args);
  if(!response?.ok)throw new Error(response?.error?.message||'The recovery request could not finish.');
  return response.data;
}
async function refresh(){
  try{
    const state=await command('get_recovery_state');
    document.getElementById('message').textContent=state.message||'Choose a queue recovery action.';
    document.getElementById('folder').textContent=state.data_directory;
    const select=document.getElementById('backup');
    select.replaceChildren(new Option('Choose a backup',''));
    for(const backup of state.backups)select.add(new Option(`${backup.name} (${Math.round(backup.size_bytes/1024)} KiB)`,backup.name));
  }catch(error){result.textContent=error.message;}
}
document.getElementById('restore').addEventListener('submit',async event=>{
  event.preventDefault();const button=event.submitter;button.disabled=true;
  try{
    const restored=await command('restore_metadata',{backupName:document.getElementById('backup').value,confirm:document.getElementById('confirm').checked});
    result.textContent=`Restored ${restored.restored_tasks} task records. Safety copy: ${restored.safety_backup}. Restart when ready.`;
  }catch(error){result.textContent=error.message;}finally{button.disabled=false;}
});
document.getElementById('refresh').addEventListener('click',refresh);
document.getElementById('retry').addEventListener('click',async()=>{try{await command('restart_application');}catch(error){result.textContent=error.message;}});
document.getElementById('network').addEventListener('click',async()=>{
  if(!window.confirm('Reset the saved proxy only? Queue and download files will be kept.'))return;
  try{await command('reset_startup_network',{confirm:true});result.textContent='Saved proxy reset. Restart when ready.';}catch(error){result.textContent=error.message;}
});
refresh();
