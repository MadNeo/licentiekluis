/* Shared browser state and LKV1 backup codec. No credentials are persisted. */
(function(root){
'use strict';
const clone=x=>JSON.parse(JSON.stringify(x));
const guid=x=>typeof x==='string'&&/^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/i.test(x)&&!/^0{8}-0{4}-0{4}-0{4}-0{12}$/.test(x);
function entry(kind='license'){return {id:crypto.randomUUID(),kind,name:'',ownerId:null,code:'',softwareVersion:'',notes:'',expires:null,installations:[],deleted:false,revision:0,mutationId:crypto.randomUUID()};}
function validate(e){
 if(!e||!guid(e.id)||!guid(e.mutationId)||!Number.isSafeInteger(e.revision)||e.revision<0||!['person','license','recovery'].includes(e.kind)||typeof e.deleted!=='boolean')throw Error('Ongeldige recordgegevens.');
 for(const [key,max] of [['name',200],['code',10000],['notes',20000],['softwareVersion',200]])if(typeof e[key]!=='string'||e[key].length>max)throw Error('Ongeldige of te lange tekst.');
 if(!e.name.trim()||(e.kind==='license'&&!guid(e.ownerId)))throw Error('Vul de naam en gebruiker in.');
 if(e.kind==='recovery')recoveryValidate(e.recovery,e.deleted);else if(e.recovery!=null)throw Error('Herstelgegevens horen bij een herstelsleutel.');
 if(e.expires!==null&&(!/^\d{4}-\d{2}-\d{2}$/.test(e.expires)||!Number.isFinite(Date.parse(e.expires))||new Date(e.expires).toISOString().slice(0,10)!==e.expires||e.expires<'0001-01-01'))throw Error('Ongeldige einddatum.');
 if(!Array.isArray(e.installations)||e.installations.length>100||e.installations.some(i=>!i||!['Windows','Linux','macOS'].includes(i.platform)||typeof i.device!=='string'||i.device.length>200))throw Error('Controleer de installaties (maximaal 100).');
}
function recoveryValidate(r,deleted=false){
 const filename=n=>typeof n==='string'&&n.trim()&&n.length<=200&&!/[\\/:\x00-\x1f\x7f-\x9f]/.test(n)&&n!=='.'&&n!=='..';
 if(!r||!['authenticator','wallet'].includes(r.category)||typeof r.text!=='string'||r.text.length>20000||r.text.includes('\0'))throw Error('Kies een categorie en gebruik maximaal 20.000 tekens hersteltekst.');
 const f=r.textFile,p=r.photo;
 if(f&&(!filename(f.name)||!f.name.toLowerCase().endsWith('.txt')||typeof f.text!=='string'||!f.text.trim()||new TextEncoder().encode(f.text).length>65536||f.text.includes('\0')))throw Error('Kies een niet-leeg UTF-8-tekstbestand (.txt, maximaal 64 KB).');
 if(p){
  if(!filename(p.name)||typeof p.data!=='string'||p.data.length>2796204)throw Error('Foto ongeldig of groter dan 2 MB.');
  let b;try{b=atob(p.data);}catch{throw Error('Ongeldige foto.');}
  if(b.length>2097152||btoa(b)!==p.data||!(p.mediaType==='image/png'&&b.startsWith('\x89PNG\r\n\x1a\n')||p.mediaType==='image/jpeg'&&b.startsWith('\xff\xd8\xff')))throw Error('Kies een PNG- of JPEG-foto (maximaal 2 MB).');
 }
 if(!deleted&&!r.text.trim()&&!f&&!p)throw Error('Voeg tekst, een tekstbestand of een foto toe. Eén vorm is voldoende.');
}
function recoveryTotal(records){const bytes=new TextEncoder();const size=[...records].reduce((n,e)=>n+(e.recovery?bytes.encode(e.recovery.text||'').length+bytes.encode(e.recovery.textFile?.text||'').length+(e.recovery.photo?.data.length||0):0),0);if(size>8*1024*1024)throw Error('Herstelgegevens samen maximaal 8 MB. Verwijder een ongebruikte foto of kies een kleinere foto.');}
function normalize(value){
 if(Array.isArray(value))return value.map(normalize);
 if(value&&typeof value==='object'){const out={};for(const [k,v] of Object.entries(value)){if(['__proto__','constructor','prototype'].includes(k))throw Error('Ongeldige back-up.');out[k[0].toLowerCase()+k.slice(1)]=normalize(v);}return out;}
 return value;
}
function importState(value){
 const data=normalize(value);if(!data||!Array.isArray(data.entries)||data.entries.length>10000)throw Error('Geen ondersteunde licentieback-up (maximaal 10.000 records).');
 const map=new Map();for(const e of data.entries){validate(e);if(map.has(e.id))throw Error('Dubbele records in back-up.');map.set(e.id,e);}
 for(const e of map.values())if(e.kind==='license'&&!e.deleted&&map.get(e.ownerId)?.kind!=='person')throw Error('Een gebruiker ontbreekt in de back-up.');
 recoveryTotal(data.entries);return data.entries;
}
class State {
 constructor(vaultId){this.vaultId=vaultId;this.records=new Map();this.pending=new Map();this.conflicts=new Map();this.lastSync=null;}
 edit(value){validate(value);const e=clone(value);recoveryTotal([...this.records.values()].filter(x=>x.id!==e.id).concat(e));if(!this.pending.has(e.id))this.pending.set(e.id,this.records.get(e.id)?.revision||0);e.mutationId=crypto.randomUUID();this.records.set(e.id,e);}
 request(){return {vaultId:this.vaultId,changes:[...this.pending].filter(([id])=>!this.conflicts.has(id)).map(([id,baseRevision])=>({value:clone(this.records.get(id)),baseRevision}))};}
 merge(response,sent){
 for(const e of response.entries)validate(e);
 const sentMap=new Map(sent.changes.map(c=>[c.value.id,c.value]));const applied=new Set(response.applied);
 for(const remote of response.entries){
  const local=this.records.get(remote.id),old=sentMap.get(remote.id);
  if(!this.pending.has(remote.id)){this.records.set(remote.id,clone(remote));continue;}
  if(applied.has(remote.id)&&old){
   this.conflicts.delete(remote.id);
   if(local.mutationId===old.mutationId){this.pending.delete(remote.id);this.records.set(remote.id,clone(remote));}
   else {local.revision=remote.revision;this.pending.set(remote.id,remote.revision);}
  }else if(remote.revision!==this.pending.get(remote.id))this.conflicts.set(remote.id,clone(remote));
 }
 this.lastSync=new Date().toISOString();
 }
 resolve(id,local){const remote=this.conflicts.get(id);if(!remote)throw Error('Conflict bestaat niet meer.');if(local){const e=this.records.get(id);e.revision=remote.revision;e.mutationId=crypto.randomUUID();this.pending.set(id,remote.revision);}else{this.records.set(id,clone(remote));this.pending.delete(id);}this.conflicts.delete(id);}
 restore(records){
  // Merge import: retain records not present in the backup, including history.
  for(const e of records){validate(e);const old=this.records.get(e.id);if(old&&old.kind!==e.kind)throw Error('Recordtype verschilt; herstel afgebroken.');}
  const combined=new Map(this.records);for(const e of records)combined.set(e.id,e);recoveryTotal(combined.values());
  for(const e of records)this.edit({...e,revision:this.records.get(e.id)?.revision||0});
 }
 backup(){return {serverUrl:'',vaultId:this.vaultId,entries:[...this.records.values()],pending:Object.fromEntries(this.pending),conflicts:Object.fromEntries(this.conflicts),lastSync:this.lastSync};}
}
async function key(password,salt){const raw=await crypto.subtle.importKey('raw',new TextEncoder().encode(password),'PBKDF2',false,['deriveKey']);return crypto.subtle.deriveKey({name:'PBKDF2',salt,iterations:600000,hash:'SHA-256'},raw,{name:'AES-GCM',length:256},false,['encrypt','decrypt']);}
async function encrypt(value,password){
 if(password.length<14)throw Error('Gebruik minimaal 14 tekens voor het back-upwachtwoord.');
 const header=new Uint8Array(32);header.set(new TextEncoder().encode('LKV1'));crypto.getRandomValues(header.subarray(4,20));crypto.getRandomValues(header.subarray(20));
 const plain=new TextEncoder().encode(JSON.stringify(value));try{const encrypted=new Uint8Array(await crypto.subtle.encrypt({name:'AES-GCM',iv:header.slice(20),additionalData:header.slice(0,20),tagLength:128},await key(password,header.slice(4,20)),plain));const out=new Uint8Array(32+encrypted.length);out.set(header);out.set(encrypted.slice(-16),32);out.set(encrypted.slice(0,-16),48);return out;}finally{plain.fill(0);}
}
async function decrypt(bytes,password){
 if(bytes.length<48||bytes.length>32*1024*1024||new TextDecoder().decode(bytes.slice(0,4))!=='LKV1')throw Error('Geen ondersteunde LKV1-back-up (maximaal 32 MB).');
 const packed=new Uint8Array(bytes.length-32);packed.set(bytes.slice(48));packed.set(bytes.slice(32,48),bytes.length-48);
 let plain;try{plain=new Uint8Array(await crypto.subtle.decrypt({name:'AES-GCM',iv:bytes.slice(20,32),additionalData:bytes.slice(0,20),tagLength:128},await key(password,bytes.slice(4,20)),packed));return JSON.parse(new TextDecoder().decode(plain));}catch{throw Error('Onjuist wachtwoord of beschadigde back-up.');}finally{plain?.fill(0);}
}
async function cacheCipher(password){
 const salt=crypto.getRandomValues(new Uint8Array(16)),derived=await key(password,salt);
 return async value=>{const header=new Uint8Array(32);header.set(new TextEncoder().encode('LKV1'));header.set(salt,4);crypto.getRandomValues(header.subarray(20));const plain=new TextEncoder().encode(JSON.stringify(value));try{const encrypted=new Uint8Array(await crypto.subtle.encrypt({name:'AES-GCM',iv:header.slice(20),additionalData:header.slice(0,20)},derived,plain));const out=new Uint8Array(32+encrypted.length);out.set(header);out.set(encrypted.slice(-16),32);out.set(encrypted.slice(0,-16),48);return out;}finally{plain.fill(0);}};
}
function recover(value,vaultId){const data=normalize(value);const records=importState(data);if(data.vaultId!==vaultId)throw Error('Herstelgegevens horen bij een andere serverkluis.');const state=new State(vaultId);state.records=new Map(records.map(e=>[e.id,e]));for(const [id,rev] of Object.entries(data.pending||{})){if(!state.records.has(id)||!Number.isSafeInteger(rev)||rev<0)throw Error('Ongeldige wachtrij.');state.pending.set(id,rev);}for(const [id,e] of Object.entries(data.conflicts||{})){validate(e);if(id!==e.id||!state.pending.has(id))throw Error('Ongeldig conflict.');state.conflicts.set(id,e);}state.lastSync=data.lastSync;return state;}
root.Vault={recoveryValidate,recoveryTotal,State,entry,validate,importState,encrypt,decrypt,clone,cacheCipher,recover};
})(typeof globalThis!=='undefined'?globalThis:this);
