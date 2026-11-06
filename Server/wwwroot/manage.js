'use strict';
let vault=null,syncWork=null,syncTimer=null,editor=null,restoreRecords=null,backupBusy=false,cacheSeal=null,cacheKey=null,cacheWork=Promise.resolve(),restoreBaseline=null; 
async function manageLogin(result,password){
 const g=generation,key='lkv-pending-'+result.vaultId;let restored=new Vault.State(result.vaultId);const saved=sessionStorage.getItem(key);
 if(saved){try{restored=Vault.recover(await Vault.decrypt(Uint8Array.from(atob(saved),c=>c.charCodeAt(0)),password),result.vaultId);}catch{throw Error('Wachtende wijzigingen konden niet worden ontsleuteld. Bewaar dit tabblad; gebruik het oorspronkelijke serverwachtwoord voor herstel.');}}
 const seal=await Vault.cacheCipher(password);if(g!==generation)return;
 vault=restored;cacheKey=key;cacheSeal=seal;scheduleSync();
}
function persistPending(current,g){
 const job=cacheWork.catch(()=>{}).then(()=>writePending(current,g));cacheWork=job;return job;
}
async function writePending(current,g){
 if(g!==generation||current!==vault)return;
 if(!cacheSeal)throw Error('Beveiligde opslag is niet beschikbaar.');const key=cacheKey;
 if(!current.pending.size){if(g===generation&&current===vault)sessionStorage.removeItem(key);return;}
 const bytes=await cacheSeal(current.backup());if(g!==generation||current!==vault)return;
 let encoded='';for(let i=0;i<bytes.length;i+=8192)encoded+=String.fromCharCode(...bytes.subarray(i,i+8192));sessionStorage.setItem(key,btoa(encoded));
}
function manageLock(){cacheSeal=null;cacheKey=null;vault=null;editor=null;restoreRecords=null;clearTimeout(syncTimer);syncTimer=null;syncWork=null;for(const id of ['editDialog','peopleDialog','backupDialog','conflictDialog']){const d=$(id);if(d.open)d.close();}$("editForm").reset();$("peopleList").replaceChildren();$("conflictList").replaceChildren();$("installRows").replaceChildren();$("backupPassword").value='';$("backupRepeat").value='';$("backupFile").value='';$("restorePreview").textContent='';}
function scheduleSync(){clearTimeout(syncTimer);syncTimer=setTimeout(async()=>{if(token&&vault)await refresh();if(token&&vault)scheduleSync();},30000);}
function projectVault(){
 if(!vault)return;entries=[...vault.records.values()].filter(e=>!e.deleted).map(e=>({...e,code:''}));
 const selected=$("owner").value;$("owner").replaceChildren(new Option('Alle gebruikers',''));
 entries.filter(e=>e.kind==='person').sort((a,b)=>a.name.localeCompare(b.name)).forEach(e=>$("owner").add(new Option(e.name,e.id)));$("owner").value=selected;
 render();if($("peopleDialog").open)renderPeople();if($("conflictDialog").open)renderConflicts();
 const count=vault.pending.size;$("syncStatus").textContent=count?`${count} wijziging(en) wachten op synchronisatie. Houd dit tabblad open of maak een back-up.`:vault.lastSync?`Gesynchroniseerd om ${new Date(vault.lastSync).toLocaleTimeString('nl-NL')}. Automatische controle elke 30 seconden.`:'Verbinden met de server…';
 $("conflicts").textContent=`Conflicten (${vault.conflicts.size})`;
}
async function manageRefresh(){
 if(!vault)return;if(syncWork){await persistPending(vault,generation);return syncWork;}
 const current=vault,g=generation,auth=token,sent=current.request();
 const work=(async()=>{try{
  await persistPending(current,g);if(g!==generation||current!==vault)return;
  const r=await fetch('/api/sync',{method:'POST',headers:{Authorization:`Bearer ${auth}`,'Content-Type':'application/json'},cache:'no-store',body:JSON.stringify(sent),signal:AbortSignal.timeout(20000)});
  if(g!==generation||current!==vault)return;
  if(r.status===401){message('Sessie verlopen. Maak bij wachtende wijzigingen eerst een back-up, log daarna opnieuw in.');throw Error('Je sessie is verlopen. Log opnieuw in.');}
  if(!r.ok){let detail;try{detail=(await r.json()).error;}catch{}throw Error(detail||'Synchroniseren mislukt. Probeer opnieuw.');}
  const response=await r.json();if(g!==generation||current!==vault)return;
  current.merge(response,sent);await persistPending(current,g);if(g!==generation||current!==vault)return;projectVault();
  if(current.conflicts.size)message('Er zijn gelijktijdige wijzigingen. Open Conflicten en kies welke versie je wilt behouden.');
 }catch(e){if(g===generation&&current===vault){projectVault();message(`${e.message} Houd dit tabblad open en maak zo nodig een versleutelde back-up.`);}}
 })();syncWork=work;try{await work;}finally{if(syncWork===work)syncWork=null;}
}
function manageControls(card,e){const actions=node('div',undefined,'actions');const edit=node('button','Bewerken'),remove=node('button','Verwijderen');edit.addEventListener('click',()=>openEditor(e.id));remove.addEventListener('click',()=>removeRecord(e.id));actions.append(edit,remove);card.append(actions);if(vault?.pending.has(e.id))card.append(node('p',vault.conflicts.has(e.id)?'Conflict: kies welke versie je behoudt.':'Wacht op synchronisatie','pending'));}
function alertError(error,target='message'){$(target).textContent=error.message||String(error);}
function installRow(value={platform:'Windows',device:''}){const row=node('div',undefined,'installation-row'),platform=node('select'),device=node('input'),remove=node('button','Verwijderen');platform.setAttribute('aria-label','Platform');for(const p of ['Windows','Linux','macOS'])platform.add(new Option(p,p));platform.value=value.platform;device.value=value.device;device.maxLength=200;device.placeholder='Apparaatnaam';device.setAttribute('aria-label','Apparaatnaam');remove.type='button';remove.addEventListener('click',()=>row.remove());row.append(platform,device,remove);$("installRows").append(row);}
function openEditor(id){
 if(!vault)return;const e=id?vault.records.get(id):Vault.entry();if(!e||e.deleted)return;editor=Vault.clone(e);
 $("editTitle").textContent=id?'Licentie bewerken':'Nieuwe licentie';$("editError").textContent='';$("licenseName").value=e.name;$("licenseCode").value=e.code;$("licenseCode").type='password';$("editShowCode").textContent='Code tonen';$("licenseVersion").value=e.softwareVersion;$("licenseExpires").value=e.expires||'';$("licenseNotes").value=e.notes;
 const owners=[...vault.records.values()].filter(p=>p.kind==='person'&&(!p.deleted||p.id===e.ownerId));$("licenseOwner").replaceChildren(new Option('Kies een gebruiker',''));owners.sort((a,b)=>a.name.localeCompare(b.name)).forEach(p=>$("licenseOwner").add(new Option(p.name+(p.deleted?' (verwijderd)':''),p.id)));$("licenseOwner").value=e.ownerId||$("owner").value||'';
 $("installRows").replaceChildren();e.installations.forEach(installRow);$("editDialog").showModal();
}
function stageEditor(){
 if(!vault||!editor)return;const e={...editor,name:$("licenseName").value.trim(),ownerId:$("licenseOwner").value||null,code:$("licenseCode").value,softwareVersion:$("licenseVersion").value,expires:$("licenseExpires").value||null,notes:$("licenseNotes").value,installations:[...$("installRows").children].map(r=>({platform:r.children[0].value,device:r.children[1].value}))};
 // An editor keeps its opening revision. A refresh must never silently replace it.
 Vault.validate(e);const latest=vault.records.get(e.id);if(latest&&latest.mutationId!==editor.mutationId){vault.edit(e);vault.pending.set(e.id,editor.revision);vault.conflicts.set(e.id,Vault.clone(latest));}else vault.edit(e);
 $("editDialog").close();editor=null;$("editForm").reset();projectVault();refresh();
}
async function removeRecord(id){const e=vault?.records.get(id);if(!e)return;const detail=e.kind==='person'?' Bestaande licenties blijven behouden en kunnen aan een andere gebruiker worden toegewezen.':'';if(!confirm(`“${e.name}” verwijderen?${detail}`))return;vault.edit({...e,deleted:true});projectVault();await refresh();}
function renderPeople(){const list=$("peopleList");list.replaceChildren();for(const e of [...vault.records.values()].filter(e=>e.kind==='person'&&!e.deleted).sort((a,b)=>a.name.localeCompare(b.name))){const row=node('div',undefined,'person-row'),name=node('input'),rename=node('button','Hernoemen'),remove=node('button','Verwijderen');name.value=e.name;name.maxLength=200;name.setAttribute('aria-label',`Naam van ${e.name}`);rename.addEventListener('click',async()=>{try{vault.edit({...vault.records.get(e.id),name:name.value.trim()});projectVault();await refresh();}catch(err){alertError(err,'peopleError');}});remove.addEventListener('click',()=>removeRecord(e.id));row.append(name,rename,remove);list.append(row);}if(!list.children.length)list.append(node('p','Voeg eerst een gebruiker toe. Daarna kun je licenties toewijzen.'));}
function renderConflicts(){const list=$("conflictList");list.replaceChildren();for(const [id,remote] of vault.conflicts){const local=vault.records.get(id),section=node('section',undefined,'card');section.append(node('h3',local.name));const preview=e=>JSON.stringify({...e,code:e.code?'[afgeschermd]':'',mutationId:undefined,id:undefined},null,2);const columns=node('div',undefined,'conflict-columns');for(const [title,e,keep] of [['Mijn wijziging',local,true],['Serverversie',remote,false]]){const c=node('div'),button=node('button',keep?'Mijn wijziging behouden':'Serverversie behouden');c.append(node('h4',title),node('pre',preview(e)));button.addEventListener('click',async()=>{vault.resolve(id,keep);projectVault();await refresh();});c.append(button);columns.append(c);}section.append(columns);if(local.code!==remote.code)section.append(node('p','Ook de licentiecode verschilt tussen deze versies.'));list.append(section);}if(!list.children.length)list.append(node('p','Er zijn geen conflicten.'));}
function openBackup(){restoreRecords=null;$("backupError").textContent='';$("restorePreview").textContent='';$("restoreApply").disabled=true;$("backupPassword").value='';$("backupRepeat").value='';$("backupFile").value='';$("backupDialog").showModal();}
async function backupAction(fn){if(backupBusy)return;backupBusy=true;$("backupExport").disabled=true;$("restoreRead").disabled=true;$("backupError").textContent='';try{await fn();}catch(e){alertError(e,'backupError');}finally{backupBusy=false;$("backupExport").disabled=false;$("restoreRead").disabled=false;}}
$("newLicense").addEventListener('click',()=>openEditor());
$("managePeople").addEventListener('click',()=>{$("peopleError").textContent='';renderPeople();$("peopleDialog").showModal();});
$("newPersonForm").addEventListener('submit',async ev=>{ev.preventDefault();try{vault.edit({...Vault.entry('person'),name:$("personName").value.trim()});$("personName").value='';projectVault();await refresh();}catch(e){alertError(e,'peopleError');}});
$("editForm").addEventListener('submit',ev=>{ev.preventDefault();try{stageEditor();}catch(e){alertError(e,'editError');}});
$("addInstall").addEventListener('click',()=>{if($("installRows").children.length>=100){$("editError").textContent='Maximaal 100 installaties.';return;}installRow();});
$("editShowCode").addEventListener('click',()=>{const hidden=$("licenseCode").type==='password';$("licenseCode").type=hidden?'text':'password';$("editShowCode").textContent=hidden?'Code verbergen':'Code tonen';});
$("editDialog").addEventListener('close',()=>{editor=null;$("editForm").reset();$("installRows").replaceChildren();});
for(const [button,dialog] of [['editCancel','editDialog'],['peopleClose','peopleDialog'],['backupClose','backupDialog'],['conflictsClose','conflictDialog']])$(button).addEventListener('click',()=>$(dialog).close());
$("conflicts").addEventListener('click',()=>{renderConflicts();$("conflictDialog").showModal();});
$("backupOpen").addEventListener('click',openBackup);
$("backupDialog").addEventListener('close',()=>{restoreRecords=null;$("backupPassword").value='';$("backupRepeat").value='';$("backupFile").value='';$("restorePreview").textContent='';});
$("backupExport").addEventListener('click',()=>backupAction(async()=>{const current=vault,g=generation,password=$("backupPassword").value;if(password!==$("backupRepeat").value)throw Error('Herhaal hetzelfde back-upwachtwoord.');const data=await Vault.encrypt(current.backup(),password);if(g!==generation||current!==vault||!$("backupDialog").open)return;const url=URL.createObjectURL(new Blob([data],{type:'application/octet-stream'})),a=node('a');a.href=url;a.download=`LicentieKluis-${new Date().toISOString().slice(0,10)}.lkv`;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);$("backupError").textContent='Versleutelde back-up aangeboden als download. Bewaar het wachtwoord apart.';}));
$("restoreRead").addEventListener('click',()=>backupAction(async()=>{restoreRecords=null;$("restoreApply").disabled=true;$("restorePreview").textContent='';const file=$("backupFile").files[0];if(!file||file.size>32*1024*1024)throw Error('Kies een .lkv-back-up van maximaal 32 MB.');const g=generation,bytes=new Uint8Array(await file.arrayBuffer()),data=await Vault.decrypt(bytes,$("backupPassword").value);const records=Vault.importState(data);if(g!==generation||!vault||!$("backupDialog").open)return;restoreRecords=records;restoreBaseline=new Map([...vault.records].map(([id,e])=>[id,Vault.clone(e)]));$("restorePreview").textContent=`${records.filter(e=>e.kind==='person'&&!e.deleted).length} gebruikers, ${records.filter(e=>e.kind==='license'&&!e.deleted).length} licenties, ${records.filter(e=>e.deleted).length} verwijderde records. Bestaande records met hetzelfde ID worden vervangen; overige gegevens blijven behouden. Maak eerst een back-up van de huidige kluis.`;$("restoreApply").disabled=false;}));
$("restoreApply").addEventListener('click',async()=>{if(!restoreRecords||!vault)return;if(!confirm('Deze gecontroleerde back-up samenvoegen met de huidige kluis?'))return;try{const changed=restoreRecords.filter(e=>{const old=restoreBaseline.get(e.id),now=vault.records.get(e.id);return old?.mutationId!==now?.mutationId;}).map(e=>({id:e.id,base:restoreBaseline.get(e.id)?.revision||0,remote:Vault.clone(vault.records.get(e.id))}));vault.restore(restoreRecords);for(const c of changed){vault.pending.set(c.id,c.base);vault.conflicts.set(c.id,c.remote);}restoreRecords=null;restoreBaseline=null;$("backupDialog").close();projectVault();await refresh();}catch(e){alertError(e,'backupError');}});
window.addEventListener('online',()=>{if(token&&vault)refresh();});
window.addEventListener('beforeunload',ev=>{if(vault?.pending.size){ev.preventDefault();ev.returnValue='';}});
