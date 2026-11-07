// UI logic tests with a minimal DOM and mocked API; not a browser/network test.
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict'),path=require('node:path');
class Element{
 constructor(tag='div',text=''){this.tagName=tag;this.textContent=text;this.value='';this.children=[];this.listeners={};this.hidden=false;this.disabled=false;this.isConnected=true;}
 append(...items){this.children.push(...items);} replaceChildren(...items){for(const c of this.children)c.isConnected=false;this.children=items;}
 add(item){this.children.push(item);} addEventListener(name,fn){this.listeners[name]=fn;}
}
const elements=new Map(),calls=[],timeouts=new Map();let timerId=0,unauthorized=false;
const people=[{id:'p1',kind:'person',name:'Gebruiker A'},{id:'p2',kind:'person',name:'Gebruiker B'}];
const licenses=[{id:'l1',kind:'license',ownerId:'p1',name:'<img src=x onerror=alert(1)>',softwareVersion:'1',notes:'Test',installations:[{platform:'Windows',device:'PC1'}],expires:null},{id:'l2',kind:'license',ownerId:'p2',name:'Serverpakket',softwareVersion:'',notes:'',installations:[{platform:'Linux',device:'LXC1'}],expires:'2026-01-01'}];
const context={console,Date,Map,Set,Option:class extends Element{constructor(t,v){super('option',t);this.value=v;}},document:{getElementById:id=>{if(!elements.has(id))elements.set(id,new Element());return elements.get(id);},createElement:tag=>new Element(tag),addEventListener(){}},window:{addEventListener(){}},location:{protocol:'https:',hostname:'server.lan'},navigator:{clipboard:{writeText:async()=>{}}},setTimeout:(fn,ms)=>{timeouts.set(++timerId,{fn,ms});return timerId;},clearTimeout:id=>timeouts.delete(id),fetch:async(url,options)=>{calls.push({url,options});if(url==='/api/login')return{ok:true,status:200,json:async()=>({token:'synthetic-token'})};if(url==='/api/logout')return{ok:true,status:200};if(unauthorized)return{ok:false,status:401};if(url==='/api/entries')return{ok:true,status:200,json:async()=>[...people,...licenses]};if(url.endsWith('/code'))return{ok:true,status:200,json:async()=>({code:'DEMO-CODE'})};throw Error('Unexpected API');}};
vm.createContext(context);vm.runInContext(fs.readFileSync(path.join(__dirname,'../Server/wwwroot/app.js'),'utf8'),context);
const el=id=>context.document.getElementById(id),run=code=>vm.runInContext(code,context);
(async()=>{
 el('username').value='demo';el('password').value='synthetic-password';await el('loginForm').listeners.submit({preventDefault(){}});
 assert.equal(el('dashboard').hidden,false);assert.equal(el('password').value,'');assert.equal(el('licenses').children.length,2);console.log('PASS: login, password clearing and list loading');
 assert.equal(el('licenses').children[0].children[0].textContent,licenses[0].name);assert.equal(el('licenses').children[0].children[0].children.length,0);console.log('PASS: user content rendered as text');
 el('platform').value='Linux';el('platform').listeners.change();assert.equal(el('licenses').children.length,1);assert.equal(el('licenses').children[0].children[0].textContent,'Serverpakket');console.log('PASS: platform filter');
 el('platform').value='';el('owner').value='p1';el('owner').listeners.change();assert.equal(el('licenses').children.length,1);console.log('PASS: owner filter');
 el('owner').value='';el('search').value='LXC1';el('search').listeners.input();assert.equal(el('licenses').children[0].children[0].textContent,'Serverpakket');console.log('PASS: device search');
 el('search').value='1';el('search').listeners.input();assert.equal(el('licenses').children[0].children[0].textContent,licenses[0].name);
 el('search').value='';const today=run('dateKey(new Date())');const previousExpiry=licenses[1].expires;licenses[1].expires=today;
 el('expiryFilter').value='soon';el('expiryFilter').listeners.change();assert.equal(el('licenses').children.length,1);assert.equal(el('licenses').children[0].children[0].textContent,'Serverpakket');
 el('expiryFilter').value='none';el('expiryFilter').listeners.change();assert.equal(el('licenses').children.length,1);assert.equal(el('licenses').children[0].children[0].textContent,licenses[0].name);
 licenses[1].expires=previousExpiry;el('clearFilters').listeners.click();assert.equal(el('licenses').children.length,2);assert.equal(el('clearFilters').disabled,true);
 el('search').value='LXC1';el('search').listeners.input();console.log('PASS: version search, expiry filters and reset');
 const card=el('licenses').children[0],actions=card.children.at(-1),code=card.children.at(-2);await actions.children[0].listeners.click();assert.equal(code.textContent,'DEMO-CODE');const reset=[...timeouts.values()].find(t=>t.ms===30000);assert.ok(reset);reset.fn();assert.equal(code.textContent,'•••• •••• ••••');console.log('PASS: authenticated reveal and timed masking');
 await run('logout()');assert.equal(el('dashboard').hidden,true);assert.equal(el('licenses').children.length,0);assert.equal(run('token'),'');assert.equal(run('entries.length'),0);console.log('PASS: logout removes credentials and rendered data');
 unauthorized=true;await el('loginForm').listeners.submit({preventDefault(){}});assert.equal(el('dashboard').hidden,true);assert.equal(run('token'),'');console.log('PASS: expired session locks browser');
 assert.ok(calls.filter(c=>c.url==='/api/entries').every(c=>c.options.headers.Authorization==='Bearer synthetic-token'));console.log('9 web UI logic checks passed (mocked DOM/API).');
})().catch(e=>{console.error(e);process.exitCode=1;});
