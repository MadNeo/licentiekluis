// Actual Chromium UI, synthetic API fixtures; no email is delivered.
const {chromium}=require('playwright'),http=require('node:http'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{randomUUID}=require('node:crypto');
const root=path.join(__dirname,'../Server/wwwroot'),records=new Map(),vaultId=randomUUID();let offline=false,requests=0;
const server=http.createServer(async(req,res)=>{
 const json=(value,status=200)=>{res.writeHead(status,{'Content-Type':'application/json'});res.end(JSON.stringify(value));};
 if(req.url.startsWith('/api/')){if(offline){res.destroy();return;}let body='';for await(const part of req)body+=part;
 if(req.url==='/api/login')return json({token:'synthetic-test-token',vaultId});
 if(req.url==='/api/logout')return json({});
 if(req.url==='/api/sync'){requests++;const data=JSON.parse(body),applied=[],conflicts=[];assert.equal(data.vaultId,vaultId);for(const c of data.changes){const old=records.get(c.value.id);if(old?.mutationId===c.value.mutationId){applied.push(c.value.id);continue;}if((old?.revision||0)!==c.baseRevision){conflicts.push(c.value.id);continue;}records.set(c.value.id,{...c.value,revision:(old?.revision||0)+1});applied.push(c.value.id);}return json({entries:[...records.values()],applied,conflicts});}
 if(req.url==='/api/settings')return json({version:'0.8.0',account:'test',mail:{configured:true,host:'smtp.example.test',port:587,from:'vault@example.test',username:'test',passwordSaved:true,revision:'fixture'}});
 if(req.url==='/api/mail/status')return json({configured:true});
 if(req.url.endsWith('/code'))return json({code:records.get(req.url.split('/')[3]).code});
 return json({},404);
 }
 const file=['/','/settings'].includes(req.url)?'index.html':req.url==='/changelog'?'changelog.html':req.url.slice(1);if(file==='changelog.txt'){res.setHeader('Content-Type','text/plain');res.end(fs.readFileSync(path.join(root,'../..','CHANGELOG.md')));return;}if(!['index.html','app.js','vault.js','manage.js','mail.js','app.css','theme.js','changelog.js','changelog.html','settings.js'].includes(file)){res.writeHead(404);res.end();return;}
 res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html');res.setHeader('Content-Security-Policy',"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'");res.end(fs.readFileSync(path.join(root,file)));
});
(async()=>{
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
 try{const page=await browser.newPage({viewport:{width:1280,height:900}}),errors=[];const waitFor=async predicate=>{const end=Date.now()+20000;while(Date.now()<end){if(await page.evaluate(predicate))return;await page.waitForTimeout(50);}throw Error('Browser state did not settle: '+predicate);};page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
 const url=`http://127.0.0.1:${server.address().port}`;await page.goto(url);await page.screenshot({path:'/tmp/licentiekluis-web-login.png',fullPage:true});

 await page.getByRole('link',{name:'Changelog',exact:true}).click();await waitFor(()=>document.getElementById('changelogContent').textContent.includes('0.7.0'));await page.getByRole('link',{name:'Terug naar de kluis'}).click();

 const login=async()=>{await page.fill('#username','test');await page.fill('#password','synthetic-test-password');await page.click('#submit');await waitFor(()=>document.querySelector('#dashboard').hidden===false);};
 const settled=()=>waitFor(()=>typeof vault!=='undefined'&&vault&&!syncWork&&vault.pending.size===0);
 await login();await settled();
 const toggleTheme=async()=>{await page.click('#settingsOpen');await page.waitForSelector('#smtpHost:enabled');await page.click('#themeToggle');await page.click('#vaultOpen');};
 await toggleTheme();assert.equal(await page.getAttribute('html','data-theme'),'dark');
 await page.click('#managePeople');await page.fill('#personName','Gebruiker A');await page.click('#newPersonForm button');await settled();await page.click('#peopleClose');
 const person=[...records.values()][0];
 // A person name draft survives refresh; stale rename is a conflict, not an overwrite.
 await page.click('#managePeople');await page.fill('#peopleList input','Mijn gebruikersnaam');const changedPerson=records.get(person.id);records.set(person.id,{...changedPerson,name:'Naam vanuit Windows',revision:changedPerson.revision+1,mutationId:randomUUID()});await page.evaluate(()=>refresh());assert.equal(await page.inputValue('#peopleList input'),'Mijn gebruikersnaam');await page.locator('#peopleList').getByRole('button',{name:'Hernoemen'}).click();await waitFor(()=>vault.conflicts.size===1);await page.click('#peopleClose');await page.click('#conflicts');await page.getByRole('button',{name:'Mijn wijziging behouden'}).click();await settled();await page.click('#conflictsClose');assert.equal(records.get(person.id).name,'Mijn gebruikersnaam');
 await page.click('#newLicense');await page.fill('#licenseName','Office');await page.selectOption('#licenseOwner',person.id);await page.fill('#licenseCode','TEST-KEY');await page.fill('#licenseVersion','2026');await page.fill('#licenseExpires','2027-12-31');await page.fill('#licenseNotes','Testnotitie');await page.click('#addInstall');await page.fill('#installRows input','Laptop kantoor');await page.click('#editForm button[type=submit]');await settled();
 let license=[...records.values()].find(e=>e.kind==='license');assert.equal(license.code,'TEST-KEY');assert.equal(license.installations[0].device,'Laptop kantoor');
 await page.locator('.license').getByRole('button',{name:'Bewerken',exact:true}).click();await page.fill('#licenseName','Office aangepast');await page.click('#editForm button[type=submit]');await settled();assert.equal(records.get(license.id).name,'Office aangepast');
 // A background refresh while the editor is open creates a visible conflict.
 await page.locator('.license').getByRole('button',{name:'Bewerken',exact:true}).click();const remote=records.get(license.id);records.set(remote.id,{...remote,name:'Windows versie',revision:remote.revision+1,mutationId:randomUUID()});await page.evaluate(()=>refresh());await page.fill('#licenseName','Webversie');await page.click('#editForm button[type=submit]');await waitFor(()=>vault.conflicts.size===1);await page.click('#conflicts');await page.getByRole('button',{name:'Mijn wijziging behouden'}).click();await settled();await page.click('#conflictsClose');assert.equal(records.get(license.id).name,'Webversie');
 // Existing mail selection/preview still works after editing.
 await page.check('.mailCheck input');await page.fill('#mailRecipient','synthetic@example.test');await page.click('#mailPrepare');await page.waitForSelector('#mailDialog[open]');assert.match(await page.textContent('#mailPreview'),/TEST-KEY/);await page.click('#mailCancel');
 // Encrypted export/import review and merge; unrelated records survive.
 await page.click('#settingsOpen');await page.click('#backupOpen');await page.fill('#backupPassword','synthetic-backup-password');await page.fill('#backupRepeat','synthetic-backup-password');const downloadPromise=page.waitForEvent('download');await page.click('#backupExport');const download=await downloadPromise;const backupPath=await download.path();assert.equal(fs.readFileSync(backupPath).subarray(0,4).toString(),'LKV1');await page.setInputFiles('#backupFile',backupPath);await page.click('#restoreRead');await waitFor(()=>!document.querySelector('#restoreApply').disabled);await page.click('#restoreApply');await settled();await page.click('#vaultOpen');
 // Loss of network: retain changes encrypted, lock, relogin and retry.
 offline=true;await page.locator('.license').getByRole('button',{name:'Bewerken',exact:true}).click();await page.fill('#licenseNotes','Wijziging tijdens storing');await page.click('#editForm button[type=submit]');await waitFor(()=>vault.pending.size===1&&!syncWork);const stored=await page.evaluate(()=>Object.values(sessionStorage).join(''));assert.ok(stored);assert.ok(!stored.includes('Wijziging tijdens storing'));await page.click('#logout');await page.waitForSelector('#pendingRecovery button');const recoveryDownload=page.waitForEvent('download');await page.click('#pendingRecovery button');assert.equal(fs.readFileSync(await (await recoveryDownload).path()).subarray(0,4).toString(),'LKV1');offline=false;await login();await settled();assert.equal(records.get(license.id).notes,'Wijziging tijdens storing');
 // Deleting an owner retains the license and historical ownership.
 await page.click('#managePeople');await page.locator('#peopleList').getByRole('button',{name:'Verwijderen'}).click();await settled();await page.click('#peopleClose');assert.equal(records.get(person.id).deleted,true);assert.equal(records.get(license.id).deleted,false);assert.match(await page.textContent('#licenses'),/Verwijderde gebruiker/);
 // The license collection precedes sharing, so records remain the primary task.
 assert.ok(await page.evaluate(()=>!!(document.getElementById('licenses').compareDocumentPosition(document.getElementById('mailSection'))&Node.DOCUMENT_POSITION_FOLLOWING)));
 // Responsive layout and lock removes sensitive forms.
 await page.setViewportSize({width:390,height:844});await page.screenshot({path:'/tmp/licentiekluis-web-mobile.png',fullPage:true});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.setViewportSize({width:1280,height:900});await page.screenshot({path:'/tmp/licentiekluis-web-dark.png',fullPage:true});await toggleTheme();assert.equal(await page.locator('#refresh').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(255, 255, 255)');await page.screenshot({path:'/tmp/licentiekluis-web-desktop.png',fullPage:true});await toggleTheme();
 await page.locator('.license').getByRole('button',{name:'Bewerken',exact:true}).click();await page.screenshot({path:'/tmp/licentiekluis-web-editor.png',fullPage:true});await page.evaluate(()=>localLock());assert.equal(await page.inputValue('#licenseCode'),'');assert.equal(await page.locator('dialog[open]').count(),0);assert.deepEqual(errors,[]);
 console.log(`PASS: Chromium CRUD, concurrent Windows edit/conflict, mail preview, encrypted backup/restore, outage/relogin recovery, owner deletion, mobile layout and lock (${requests} sync calls)`);
 }finally{await browser.close();server.close();}
})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
