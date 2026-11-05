// UI logic tests with a minimal DOM and mocked API; not a browser/network test.
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict'),path=require('node:path');
class Element{
 constructor(tag='div',text=''){this.tagName=tag;this.textContent=text;this.value='';this.children=[];this.listeners={};this.hidden=false;this.disabled=false;this.isConnected=true;}
 append(...items){this.children.push(...items);} replaceChildren(...items){for(const c of this.children)c.isConnected=false;this.children=items;}
 showModal(){this.open=true;} close(){this.open=false;} add(item){this.children.push(item);} addEventListener(name,fn){this.listeners[name]=fn;}
}
const elements=new Map(),calls=[],timeouts=new Map();let timerId=0,unauthorized=false;
const people=[{id:'p1',kind:'person',name:'Gebruiker A'},{id:'p2',kind:'person',name:'Gebruiker B'}];
const licenses=[{id:'l1',kind:'license',ownerId:'p1',name:'<img src=x onerror=alert(1)>',softwareVersion:'1',notes:'Test',installations:[{platform:'Windows',device:'PC1'}],expires:null},{id:'l2',kind:'license',ownerId:'p2',name:'Serverpakket',softwareVersion:'',notes:'',installations:[{platform:'Linux',device:'LXC1'}],expires:'2026-01-01'}];
let failMail=true;
const context={crypto:{randomUUID:()=> 'test-request-id'},console,Date,Map,Set,Option:class extends Element{constructor(t,v){super('option',t);this.value=v;}},document:{getElementById:id=>{if(!elements.has(id))elements.set(id,new Element());return elements.get(id);},createElement:tag=>new Element(tag),addEventListener(){}},window:{addEventListener(){}},location:{protocol:'https:',hostname:'server.lan'},navigator:{clipboard:{writeText:async()=>{}}},setTimeout:(fn,ms)=>{timeouts.set(++timerId,{fn,ms});return timerId;},clearTimeout:id=>timeouts.delete(id),fetch:async(url,options)=>{calls.push({url,options});if(url==='/api/mail/status')return{ok:true,json:async()=>({configured:true})};if(url==='/api/mail/send'){if(failMail){failMail=false;throw Error('Connection lost');}return{ok:true,status:200,json:async()=>({status:'submitted',message:'Aangeboden aan mailserver'})};}if(url==='/api/login')return{ok:true,status:200,json:async()=>({token:'synthetic-token'})};if(url==='/api/logout')return{ok:true,status:200};if(unauthorized)return{ok:false,status:401};if(url==='/api/entries')return{ok:true,status:200,json:async()=>[...people,...licenses]};if(url.endsWith('/code'))return{ok:true,status:200,json:async()=>({code:'DEMO-CODE'})};throw Error('Unexpected API');}};
vm.createContext(context);vm.runInContext(fs.readFileSync(path.join(__dirname,'../Server/wwwroot/app.js'),'utf8'),context);
const el=id=>context.document.getElementById(id),run=code=>vm.runInContext(code,context);

vm.runInContext(fs.readFileSync(path.join(__dirname,"../Server/wwwroot/mail.js"),"utf8"),context);
(async()=>{
 el('username').value='demo';el('password').value='demo';await el('loginForm').listeners.submit({preventDefault(){}});
 el('mailAll').listeners.click();assert.equal(run('mailSelected.size'),2);
 el('platform').value='Linux';el('platform').listeners.change();assert.equal(run('mailSelected.size'),2);
 console.log('PASS: multiple selection survives filters');
 el('mailRecipient').value='test@example.org';el('mailSubject').value='Licenties';await el('mailPrepare').listeners.click();
 assert.ok(el('mailDialog').open);assert.ok(el('mailPreview').textContent.includes('Serverpakket'));assert.equal(el('mailPreview').textContent.split('DEMO-CODE').length,3);assert.equal(calls.filter(c=>c.url==='/api/mail/send').length,0);
 console.log('PASS: full software/code preview without sending');
 await el('mailSend').listeners.click();assert.ok(run('mailRequest'));assert.match(el('mailResult').textContent,/niet automatisch/);
 await el('mailSend').listeners.click();const sent=calls.filter(c=>c.url==='/api/mail/send').map(c=>JSON.parse(c.options.body));assert.equal(sent.length,2);assert.equal(sent[0].requestId,sent[1].requestId);assert.equal(sent[0].licenses.length,2);assert.equal(sent[0].recipient,'test@example.org');assert.equal(run('mailRequest'),null);assert.equal(el('mailPreview').textContent,'');
 console.log('PASS: explicit retry preserves request ID; completion clears preview');
 el('mailAll').listeners.click();el('mailRecipient').value='one@example.org;two@example.org';await el('mailPrepare').listeners.click();assert.equal(run('mailRequest'),null);
 console.log('PASS: multiple recipients rejected');
 el('mailRecipient').value='test@example.org';await el('mailPrepare').listeners.click();assert.ok(run('mailRequest'));await run('logout()');assert.equal(run('mailRequest'),null);assert.equal(run('mailSelected.size'),0);assert.equal(el('mailPreview').textContent,'');assert.equal(el('mailRecipient').value,'');
 console.log('PASS: logout clears selection, recipient and preview');
 console.log('5 mail UI checks passed (mocked DOM/API, no email sent).');
})().catch(e=>{console.error(e);process.exitCode=1;});
