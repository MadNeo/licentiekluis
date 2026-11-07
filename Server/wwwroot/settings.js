'use strict';
let settingsValue=null,settingsDirty=false,settingsBusy=false,settingsEpoch=0,testRequest=null;
function settingsControls(){
 $('smtpFields').disabled=settingsBusy||!settingsValue;
 $('smtpTestSend').disabled=settingsBusy||settingsDirty||!settingsValue?.configured;
 $('smtpTestRecipient').disabled=settingsBusy;
 $('settingsRetry').disabled=settingsBusy;
 $('smtpUser').disabled=!$('smtpAuth').checked;
 $('smtpPassword').disabled=!$('smtpAuth').checked;
 $('smtpUser').required=$('smtpAuth').checked;
 $('smtpPassword').required=$('smtpAuth').checked&&!settingsValue?.passwordSaved;
}
function settingsLock(){
 settingsEpoch++;settingsValue=null;settingsDirty=false;settingsBusy=false;testRequest=null;
 $('smtpForm').reset();$('smtpTestForm').reset();$('settingsResult').textContent='';$('smtpTestResult').textContent='';
 for(const id of ['settingsAccount','settingsVersion','settingsServer'])$(id).textContent='—';
 $('settingsPanel').hidden=true;$('settingsOpen').hidden=true;$('vaultOpen').hidden=true;
 $('smtpStatus').textContent='Log in om instellingen te bekijken';settingsControls();
}
function settingsRoute(){
 if(!token)return;
 const active=location.pathname==='/settings';
 const recovery=location.pathname==='/recovery';
 $('dashboard').hidden=active||recovery;if($('recoveryPanel'))$('recoveryPanel').hidden=!recovery;
 if($('recoveryOpen'))$('recoveryOpen').setAttribute('aria-current',recovery?'page':'false');$('settingsPanel').hidden=!active;
 $('settingsOpen').setAttribute('aria-current',active?'page':'false');
 $('vaultOpen').setAttribute('aria-current',active||recovery?'false':'page');
 document.title=active?'LicentieKluis · Instellingen':recovery?'LicentieKluis · Herstelsleutels':'LicentieKluis · Jouw licentieoverzicht';
 if(active&&!settingsValue&&!settingsBusy)loadSettings();
}
function settingsLogin(){
 if($('recoveryOpen'))$('recoveryOpen').hidden=false;$('settingsOpen').hidden=false;$('vaultOpen').hidden=false;settingsRoute();
}
function navigateSettings(path){
 if(settingsBusy){message('Wacht tot de huidige instelling is opgeslagen of getest.');return;}
 if(location.pathname==='/settings'&&settingsDirty&&!path.startsWith('/settings')&&!confirm('Niet-opgeslagen mailinstellingen verwerpen?'))return;
 if(!path.startsWith('/settings')){settingsDirty=false;settingsValue=null;$('smtpPassword').value='';}
 history.pushState(null,'',path);message();settingsRoute();
 if(path.startsWith('/settings'))$('settingsTitle').focus();
 window.scrollTo(0,0);
}
for(const [id,path] of [['settingsOpen','/settings'],['vaultOpen','/'],['settingsBack','/'],['mailSettingsOpen','/settings#emailSettings']]){
 $(id).addEventListener('click',event=>{if(event.ctrlKey||event.metaKey||event.shiftKey||event.altKey)return;event.preventDefault();navigateSettings(path);});
}
window.addEventListener('popstate',settingsRoute);
window.addEventListener('beforeunload',event=>{if(settingsDirty||settingsBusy){event.preventDefault();event.returnValue='';}});
async function settingsRequest(path,method='GET',body){
 const current=token,epoch=settingsEpoch;
 const response=await fetch(path,{method,cache:'no-store',headers:{Authorization:`Bearer ${current}`,...(body?{'Content-Type':'application/json'}:{})},...(body?{body:JSON.stringify(body)}:{})});
 if(current!==token||epoch!==settingsEpoch)throw Error('Sessie gewijzigd.');
 if(response.status===401){localLock();throw Error('Je sessie is verlopen. Log opnieuw in.');}
 if(response.status===429)throw Error('Te veel testmails. Wacht een minuut voordat je opnieuw probeert.');
 const result=await response.json();
 if(!response.ok)throw Error(result.error||'Bewerking niet voltooid. Controleer de serververbinding.');
 return result;
}
function populateMail(value){
 settingsValue=value;settingsDirty=false;testRequest=null;
 $('smtpHost').value=value.host;$('smtpPort').value=value.port;$('smtpFrom').value=value.from;$('smtpUser').value=value.username;
 $('smtpPassword').value='';$('smtpAuth').checked=!value.configured||Boolean(value.username);
 $('smtpPasswordHint').textContent=value.passwordSaved?'Wachtwoord opgeslagen. Laat leeg om het te behouden. Voer het opnieuw in bij een andere host, poort of gebruikersnaam.':'Gebruik zo nodig een app-wachtwoord van je mailprovider.';
 $('smtpStatus').textContent=value.configured?'Ingesteld':'Nog niet ingesteld';
 $('smtpTestSend').textContent='Testmail versturen';$('smtpTestResult').textContent='';
 if(value.configured)$('mailSettingsHint').hidden=true;
 settingsControls();
}
async function loadSettings(){
 if(settingsBusy)return;
 const epoch=settingsEpoch;settingsBusy=true;settingsControls();$('settingsRetry').hidden=true;$('settingsResult').textContent='Instellingen laden…';
 try{
  const data=await settingsRequest('/api/settings');if(epoch!==settingsEpoch)return;
  populateMail(data.mail);$('settingsServer').textContent=location.origin;$('settingsAccount').textContent=data.account;$('settingsVersion').textContent=data.version;$('settingsResult').textContent='';
 }catch(error){if(epoch===settingsEpoch){$('settingsResult').textContent=error.message;$('settingsRetry').hidden=false;}}
 finally{if(epoch===settingsEpoch){settingsBusy=false;settingsControls();}}
}
$('smtpForm').addEventListener('input',()=>{settingsDirty=true;testRequest=null;$('smtpStatus').textContent='Niet opgeslagen';$('smtpTestResult').textContent='';settingsControls();});
$('smtpAuth').addEventListener('change',()=>{if(!$('smtpAuth').checked){$('smtpUser').value='';$('smtpPassword').value='';}settingsDirty=true;settingsControls();});
$('smtpForm').addEventListener('submit',async event=>{
 event.preventDefault();if(settingsBusy||!settingsValue)return;
 const epoch=settingsEpoch,auth=$('smtpAuth').checked;
 const update={host:$('smtpHost').value.trim(),port:Number($('smtpPort').value),from:$('smtpFrom').value.trim(),username:auth?$('smtpUser').value.trim():'',password:auth?$('smtpPassword').value:'',clearPassword:!auth,revision:settingsValue.revision};
 settingsBusy=true;settingsControls();$('settingsResult').textContent='Instellingen opslaan…';
 try{const saved=await settingsRequest('/api/settings/mail','PUT',update);if(epoch!==settingsEpoch)return;populateMail(saved);$('settingsResult').textContent='Mailinstellingen opgeslagen. Ze zijn direct actief voor web en Windows. Verstuur nu een testmail.';}
 catch(error){if(epoch===settingsEpoch)$('settingsResult').textContent=error.message;}
 finally{if(epoch===settingsEpoch){settingsBusy=false;settingsControls();}}
});
$('smtpReload').addEventListener('click',()=>{if(settingsDirty&&!confirm('Je niet-opgeslagen mailinstellingen verwerpen en de opgeslagen instellingen laden?'))return;loadSettings();});
$('settingsRetry').addEventListener('click',loadSettings);
$('smtpTestRecipient').addEventListener('input',()=>{testRequest=null;$('smtpTestSend').textContent='Testmail versturen';});
$('smtpTestForm').addEventListener('submit',async event=>{
 event.preventDefault();if(settingsBusy||settingsDirty||!settingsValue?.configured)return;
 const epoch=settingsEpoch;
 if(!testRequest)testRequest={requestId:crypto.randomUUID(),recipient:$('smtpTestRecipient').value.trim(),revision:settingsValue.revision};
 settingsBusy=true;settingsControls();$('smtpTestResult').textContent='Testmail versturen…';
 try{const result=await settingsRequest('/api/settings/mail/test','POST',testRequest);if(epoch!==settingsEpoch)return;$('smtpTestResult').textContent=result.message;testRequest=null;$('smtpTestSend').textContent='Nieuwe testmail versturen';}
 catch(error){if(epoch===settingsEpoch){$('smtpTestResult').textContent=`${error.message} Er wordt niet automatisch opnieuw verstuurd.`;$('smtpTestSend').textContent='Status opnieuw controleren';}}
 finally{if(epoch===settingsEpoch){settingsBusy=false;settingsControls();}}
});
