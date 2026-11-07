'use strict';
let recoveryDraft=null,recoveryReadCount=0,recoveryEpoch=0,recoveryViewerTimer=null,recoveryTextTimer=null,recoveryUrls=[];
const recoveryLabel=kind=>kind==='wallet'?'Crypto-wallet':'Authenticator';
function recoveryHideView(){clearTimeout(recoveryViewerTimer);recoveryViewerTimer=null;$('recoveryViewBody').replaceChildren();for(const url of recoveryUrls)URL.revokeObjectURL(url);recoveryUrls=[];if($('recoveryView').open)$('recoveryView').close();}
function recoveryClearDraft(){recoveryEpoch++;recoveryDraft=null;recoveryReadCount=0;clearTimeout(recoveryTextTimer);$('recoveryForm').reset();$('recoveryText').value='';$('recoveryFile').value='';$('recoveryPhoto').value='';$('recoveryFileName').textContent='';$('recoveryPhotoName').textContent='';$('recoveryError').textContent='';for(const id of ['recoverySave','recoveryFile','recoveryPhoto'])$(id).disabled=false;}
function recoveryLock(){recoveryHideView();recoveryClearDraft();if($('recoveryDialog').open)$('recoveryDialog').close();$('recoveryList').replaceChildren();$('recoveryPanel').hidden=true;$('recoveryOpen').hidden=true;$('recoveryCount').textContent='';$('recoverySyncStatus').textContent='';}
function recoveryRender(){
 if(!vault)return;const all=[...vault.records.values()].filter(e=>e.kind==='recovery'&&!e.deleted),q=$('recoverySearch').value.trim().toLocaleLowerCase(),category=$('recoveryFilter').value;
 const rows=all.filter(e=>e.name.toLocaleLowerCase().includes(q)&&(!category||e.recovery.category===category)).sort((a,b)=>a.name.localeCompare(b.name,'nl'));
 $('recoveryCount').textContent=`${rows.length} van ${all.length} herstelsleutels`;
 $('recoverySyncStatus').textContent=vault.pending.size?`${vault.pending.size} wijziging(en) wachten op synchronisatie. Houd dit tabblad open of maak een versleutelde back-up via Instellingen.`:vault.lastSync?`Gesynchroniseerd om ${new Date(vault.lastSync).toLocaleTimeString('nl-NL')}.`:'';
 const list=$('recoveryList');list.replaceChildren();
 if(!rows.length){const empty=node('article',undefined,'card recovery-empty');empty.append(node('h3',all.length?'Geen resultaten':'Je herstelgegevens, goed opgeborgen'),node('p',all.length?'Pas je zoekopdracht of categorie aan.':'Voeg je eerste authenticator of crypto-wallet toe. Je kiest zelf of je tekst, een bestand of een foto bewaart.'));list.append(empty);return;}
 for(const e of rows){const card=node('article',undefined,'license recovery-card');card.append(node('span',recoveryLabel(e.recovery.category),'badge'),node('h3',e.name),node('p','Inhoud afgeschermd','muted'));
  const types=node('div',undefined,'recovery-types');for(const [label,present] of [['Tekst',e.recovery.text.trim()],['Tekstbestand',e.recovery.textFile],['Foto',e.recovery.photo]])if(present)types.append(node('span',label,'badge'));card.append(types);
  const actions=node('div',undefined,'actions');for(const [text,action] of [['Bekijken',()=>recoveryView(e.id)],['Bewerken',()=>recoveryEdit(e.id)],['Verwijderen',()=>recoveryDelete(e.id)]]){const b=node('button',text);b.addEventListener('click',action);actions.append(b);}card.append(actions);
  if(vault.pending.has(e.id))card.append(node('p',vault.conflicts.has(e.id)?'Conflict: kies welke versie je behoudt.':'Wacht op synchronisatie','pending'));list.append(card);
 }
}
function recoveryShowText(show){clearTimeout(recoveryTextTimer);$('recoveryText').hidden=!show;$('recoveryEditText').textContent=show?'Tekst verbergen':'Tekst tonen / bewerken';if(show)recoveryTextTimer=setTimeout(()=>recoveryShowText(false),30000);}
function recoveryAttachments(){const r=recoveryDraft.recovery;$('recoveryFileName').textContent=r.textFile?.name||'Geen tekstbestand toegevoegd';$('recoveryPhotoName').textContent=r.photo?.name||'Geen foto toegevoegd';$('recoveryFileRemove').disabled=!r.textFile;$('recoveryPhotoRemove').disabled=!r.photo;}
function recoveryEdit(id){
 if(!vault)return;recoveryHideView();recoveryClearDraft();const e=id?vault.records.get(id):{...Vault.entry('recovery'),recovery:{category:'authenticator',text:'',textFile:null,photo:null}};if(!e||e.deleted)return;
 recoveryDraft=Vault.clone(e);$('recoveryEditTitle').textContent=id?'Herstelsleutel bewerken':'Nieuwe herstelsleutel';$('recoveryName').value=e.name;$('recoveryCategory').value=e.recovery.category;$('recoveryText').value=e.recovery.text;recoveryShowText(!e.recovery.text);recoveryAttachments();$('recoveryDialog').showModal();$('recoveryName').focus();
}
async function recoveryRead(which){
 if(!recoveryDraft)return;const epoch=recoveryEpoch,draft=recoveryDraft,id=which==='photo'?'recoveryPhoto':'recoveryFile',file=$(id).files[0];if(!file)return;recoveryReadCount++;for(const control of ['recoverySave','recoveryFile','recoveryPhoto','recoveryFileRemove','recoveryPhotoRemove'])$(control).disabled=true;$('recoveryError').textContent='';
 try{
  if(file.size>(which==='photo'?2097152:65536))throw Error(which==='photo'?'Kies een foto van maximaal 2 MB.':'Kies een tekstbestand van maximaal 64 KB.');
  const bytes=new Uint8Array(await file.arrayBuffer());let value;
  if(which==='photo'){
   const mediaType=bytes[0]===137?'image/png':bytes[0]===255?'image/jpeg':'';let binary='';for(let i=0;i<bytes.length;i+=8192)binary+=String.fromCharCode(...bytes.subarray(i,i+8192));value={name:file.name,mediaType,data:btoa(binary)};
  }else{let text;try{text=new TextDecoder('utf-8',{fatal:true}).decode(bytes);}catch{throw Error('Gebruik een UTF-8-tekstbestand (.txt).');}value={name:file.name,text};}
  const candidate={...draft.recovery,[which==='photo'?'photo':'textFile']:value};Vault.recoveryValidate(candidate,true);
  if(epoch!==recoveryEpoch||draft!==recoveryDraft)return;draft.recovery=candidate;recoveryAttachments();
 }catch(e){if(epoch===recoveryEpoch)$('recoveryError').textContent=e.message;}finally{if(epoch===recoveryEpoch){recoveryReadCount--;for(const control of ['recoverySave','recoveryFile','recoveryPhoto'])$(control).disabled=recoveryReadCount>0;recoveryAttachments();$(id).value='';}}
}
function recoveryView(id){
 const e=vault?.records.get(id);if(!e||e.deleted)return;recoveryHideView();$('recoveryViewTitle').textContent=e.name;const body=$('recoveryViewBody');
 for(const [title,text] of [['Handmatig ingevoerde tekst',e.recovery.text],[e.recovery.textFile?.name,e.recovery.textFile?.text]])if(text){const section=node('section');section.append(node('h3',title),node('pre',text,'recovery-secret'));body.append(section);}
 const photo=e.recovery.photo;if(photo){const bytes=Uint8Array.from(atob(photo.data),c=>c.charCodeAt(0));const url=URL.createObjectURL(new Blob([bytes],{type:photo.mediaType}));recoveryUrls.push(url);const img=node('img');img.alt='Opgeslagen foto met herstelgegevens';img.src=url;img.className='recovery-photo';const section=node('section');section.append(node('h3',photo.name),img);body.append(section);}
 $('recoveryView').showModal();recoveryViewerTimer=setTimeout(recoveryHideView,30000);
}
async function recoveryDelete(id){const e=vault?.records.get(id);if(!e||!confirm(`Herstelsleutel “${e.name}” inclusief bijlagen verwijderen? Dit wordt ook gesynchroniseerd.`))return;try{vault.edit({...e,deleted:true,recovery:{category:e.recovery.category,text:'',textFile:null,photo:null}});projectVault();await refresh();}catch(e){message(e.message);}}
$('recoveryForm').addEventListener('submit',async event=>{event.preventDefault();if(!recoveryDraft||!vault||recoveryReadCount)return;try{
 const current=vault.records.get(recoveryDraft.id);if(current&&current.mutationId!==recoveryDraft.mutationId)throw Error('Deze herstelsleutel is ondertussen gewijzigd. Annuleer en open de actuele versie.');
 const e={...recoveryDraft,name:$('recoveryName').value.trim(),recovery:{...recoveryDraft.recovery,category:$('recoveryCategory').value,text:$('recoveryText').value}};vault.edit(e);$('recoveryDialog').close();projectVault();await refresh();
 }catch(e){$('recoveryError').textContent=e.message;}});
$('recoveryOpen').addEventListener('click',event=>{if(event.ctrlKey||event.metaKey||event.shiftKey||event.altKey)return;event.preventDefault();navigateSettings('/recovery');recoveryRender();});
$('recoveryNew').addEventListener('click',()=>recoveryEdit());$('recoverySearch').addEventListener('input',recoveryRender);$('recoveryFilter').addEventListener('change',recoveryRender);
$('recoverySync').addEventListener('click',refresh);$('recoveryConflicts').addEventListener('click',()=>{renderConflicts();$('conflictDialog').showModal();});
$('recoveryCancel').addEventListener('click',()=>$('recoveryDialog').close());$('recoveryDialog').addEventListener('close',recoveryClearDraft);
$('recoveryEditText').addEventListener('click',()=>recoveryShowText($('recoveryText').hidden));
$('recoveryFile').addEventListener('change',()=>recoveryRead('textFile'));$('recoveryPhoto').addEventListener('change',()=>recoveryRead('photo'));
for(const [id,key] of [['recoveryFileRemove','textFile'],['recoveryPhotoRemove','photo']])$(id).addEventListener('click',()=>{if(recoveryDraft){recoveryDraft.recovery[key]=null;recoveryAttachments();}});
$('recoveryViewClose').addEventListener('click',recoveryHideView);$('recoveryView').addEventListener('close',()=>{if(!$('recoveryView').open)recoveryHideView();});
document.addEventListener('visibilitychange',()=>{if(document.hidden){recoveryHideView();recoveryShowText(false);}});

$('recoveryText').addEventListener('input',()=>recoveryShowText(true));
