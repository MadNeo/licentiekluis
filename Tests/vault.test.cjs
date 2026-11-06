// Protocol and encryption tests against independent Node crypto, not a mock codec.
const assert=require('node:assert/strict'),{webcrypto,randomBytes,pbkdf2Sync,createCipheriv,createDecipheriv}=require('node:crypto');
global.crypto=webcrypto;require('../Server/wwwroot/vault.js');
const V=global.Vault,password='synthetic-test-password',id=()=>webcrypto.randomUUID();
(async()=>{
 const state=new V.State(id()),p={...V.entry('person'),name:'Persoon'};state.edit(p);
 let sent=state.request(),remote={...sent.changes[0].value,revision:1};state.edit({...state.records.get(p.id),name:'Tijdens synchronisatie'});
 state.merge({entries:[remote],applied:[p.id],conflicts:[]},sent);assert.equal(state.records.get(p.id).name,'Tijdens synchronisatie');assert.equal(state.pending.get(p.id),1);
 sent=state.request();remote={...remote,name:'Windows wijziging',revision:2,mutationId:id()};state.merge({entries:[remote],applied:[],conflicts:[p.id]},sent);assert.equal(state.conflicts.get(p.id).name,'Windows wijziging');assert.equal(state.request().changes.length,0);
 state.resolve(p.id,true);assert.equal(state.pending.get(p.id),2);sent=state.request();state.merge({entries:[{...sent.changes[0].value,revision:3}],applied:[p.id],conflicts:[]},sent);assert.equal(state.pending.size,0);
 const license={...V.entry(),name:'Office',ownerId:p.id,code:'SYNTHETIC-CODE',installations:[{platform:'Windows',device:'Werkplek'}]};state.edit(license);state.edit({...state.records.get(p.id),deleted:true});assert.equal(V.importState(state.backup()).length,2);assert.equal(state.records.get(license.id).deleted,false);
 assert.throws(()=>V.importState({entries:[license]}),/gebruiker/);assert.throws(()=>V.importState({entries:[p,p]}),/Dubbele/);assert.throws(()=>V.validate({...license,expires:'2026-02-31'}),/einddatum/);
 const before=JSON.stringify(state.backup());assert.throws(()=>state.restore([{...license,kind:'person'}]),/Recordtype/);assert.equal(JSON.stringify(state.backup()),before);
 // Browser export -> independently implemented Windows/.NET LKV1 decryption.
 const bytes=await V.encrypt(state.backup(),password);assert.ok(!Buffer.from(bytes).includes(Buffer.from('SYNTHETIC-CODE')));
 const key=pbkdf2Sync(password,bytes.slice(4,20),600000,32,'sha256'),dec=createDecipheriv('aes-256-gcm',key,bytes.slice(20,32));dec.setAAD(bytes.slice(0,20));dec.setAuthTag(bytes.slice(32,48));const plain=Buffer.concat([dec.update(bytes.slice(48)),dec.final()]);assert.equal(JSON.parse(plain).entries.length,2);
 // Independent PascalCase Windows/.NET backup -> browser restore.
 const pascal=x=>Array.isArray(x)?x.map(pascal):x&&typeof x==='object'?Object.fromEntries(Object.entries(x).map(([k,v])=>[k[0].toUpperCase()+k.slice(1),pascal(v)])):x;
 const header=Buffer.concat([Buffer.from('LKV1'),randomBytes(16),randomBytes(12)]),cipher=createCipheriv('aes-256-gcm',pbkdf2Sync(password,header.subarray(4,20),600000,32,'sha256'),header.subarray(20));cipher.setAAD(header.subarray(0,20));const body=Buffer.concat([cipher.update(JSON.stringify(pascal(state.backup()))),cipher.final()]),backup=Buffer.concat([header,cipher.getAuthTag(),body]);
 assert.equal(V.importState(await V.decrypt(backup,password))[1].code,'SYNTHETIC-CODE');await assert.rejects(()=>V.decrypt(bytes,'wrong-password'));const corrupted=bytes.slice();corrupted[50]^=1;await assert.rejects(()=>V.decrypt(corrupted,password));
 const seal=await V.cacheCipher(password),cached=await seal(state.backup()),recovered=V.recover(await V.decrypt(cached,password),state.vaultId);assert.equal(recovered.pending.size,2);assert.equal(recovered.records.get(license.id).code,license.code);assert.throws(()=>V.recover(state.backup(),id()),/andere/);
 console.log('PASS: concurrent edits, revision conflicts, tombstones, import validation, atomic restore, LKV1 cross-codec encryption, corruption rejection and encrypted pending recovery');
})().catch(e=>{console.error(e);process.exitCode=1;});
