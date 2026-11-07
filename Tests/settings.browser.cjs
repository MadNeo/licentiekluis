// Actual Chromium, synthetic settings API. No external SMTP or email.
const {chromium}=require('playwright'),http=require('node:http'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{randomUUID}=require('node:crypto');
const root=path.join(__dirname,'../Server/wwwroot'),vaultId=randomUUID();let firstLoad=true,expire=false,testCalls=[],deliveries=0;
let saved={configured:false,host:'',port:587,from:'',username:'',passwordSaved:false,revision:'new'},password='';
const server=http.createServer(async(req,res)=>{
 const json=(value,status=200)=>{res.writeHead(status,{'Content-Type':'application/json','Cache-Control':'no-store'});res.end(JSON.stringify(value));};
 if(req.url.startsWith('/api/')){
  let raw='';for await(const part of req)raw+=part;const body=raw?JSON.parse(raw):{};
  if(req.url==='/api/login')return json({token:'fixture',vaultId});
  if(expire||req.headers.authorization!=='Bearer fixture')return json({},401);
  if(req.url==='/api/sync')return json({entries:[],applied:[],conflicts:[]});
  if(req.url==='/api/logout')return json({});
  if(req.url==='/api/settings'){if(firstLoad){firstLoad=false;return json({error:'Test: tijdelijk onbereikbaar'},503);}return json({mail:saved,version:'0.8.0',account:'beheerder'});}
  if(req.url==='/api/settings/mail'){
   if(body.revision!==saved.revision)return json({error:'De instellingen zijn elders gewijzigd. Herlaad ze voordat je opnieuw opslaat.'},409);
   password=body.clearPassword?'':body.password||password;
   saved={configured:true,host:body.host,port:body.port,from:body.from,username:body.username,passwordSaved:Boolean(password),revision:randomUUID()};return json(saved);
  }
  if(req.url==='/api/settings/mail/test'){testCalls.push(body);if(testCalls.length===1){deliveries++;return json({error:'Test: antwoord na verzending onderbroken'},503);}assert.equal(body.requestId,testCalls[0].requestId);return json({status:'submitted',message:'Deze testmail is al aangeboden; niet opnieuw verzonden.'});}
  return json({},404);
 }
 const file=['/','/settings'].includes(req.url)?'index.html':req.url.slice(1);
 if(!['index.html','app.js','vault.js','manage.js','mail.js','settings.js','app.css','theme.js'].includes(file))return json({},404);
 res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html');
 res.setHeader('Content-Security-Policy',"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'");res.end(fs.readFileSync(path.join(root,file)));
});
(async()=>{
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1280,height:900}}),errors=[];const waitFor=async predicate=>{const end=Date.now()+20000;while(Date.now()<end){if(await page.evaluate(predicate))return;await page.waitForTimeout(50);}throw Error('Browser state did not settle: '+predicate);};page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
  await page.goto(`http://127.0.0.1:${server.address().port}/settings`);
  assert.equal(await page.locator('#settingsPanel').isVisible(),false);await page.fill('#username','beheerder');await page.fill('#password','synthetic-test-password');await page.click('#submit');
  await page.waitForSelector('#settingsRetry:visible');assert.equal(await page.locator('#smtpSave').isEnabled(),false);await page.click('#settingsRetry');await page.waitForSelector('#smtpHost:enabled');
  assert.equal(await page.locator('#dashboard').isVisible(),false);assert.equal(await page.locator('#smtpTestSend').isEnabled(),false);
  await page.fill('#smtpHost','smtp.example.test');await page.fill('#smtpFrom','licenties@example.test');await page.fill('#smtpUser','beheerder');await page.fill('#smtpPassword','SECRET-SMTP-PASSWORD');await page.click('#smtpSave');
  await waitFor(()=>document.querySelector('#settingsResult').textContent.includes('opgeslagen'));
  assert.equal(password,'SECRET-SMTP-PASSWORD');assert.equal(await page.inputValue('#smtpPassword'),'');assert.equal(await page.locator('#smtpTestSend').isEnabled(),true);
  assert.ok(!(await page.evaluate(()=>JSON.stringify({...localStorage,...sessionStorage}))).includes('SECRET-SMTP-PASSWORD'));
  await page.fill('#smtpFrom','nieuw@example.test');assert.equal(await page.locator('#smtpTestSend').isEnabled(),false);await page.click('#smtpSave');await page.waitForSelector('#smtpTestSend:enabled');assert.equal(password,'SECRET-SMTP-PASSWORD');
  await page.fill('#smtpTestRecipient','test@example.invalid');await page.click('#smtpTestSend');await waitFor(()=>document.querySelector('#smtpTestSend').textContent.includes('Status opnieuw'));assert.equal(deliveries,1);await page.click('#smtpTestSend');await waitFor(()=>document.querySelector('#smtpTestResult').textContent.includes('al aangeboden'));assert.equal(deliveries,1);assert.equal(testCalls.length,2);
  await page.screenshot({path:'/tmp/licentiekluis-settings-desktop.png',fullPage:true});
  await page.click('#themeToggle');await page.setViewportSize({width:390,height:844});await page.screenshot({path:'/tmp/licentiekluis-settings-mobile.png',fullPage:true});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.setViewportSize({width:1280,height:900});await page.screenshot({path:'/tmp/licentiekluis-settings-dark.png',fullPage:true});
  await page.click('#vaultOpen');assert.equal(await page.locator('#dashboard').isVisible(),true);await page.click('#settingsOpen');await page.waitForSelector('#smtpHost:enabled');assert.equal(await page.inputValue('#smtpHost'),'smtp.example.test');
  // A second administrator's changes cannot be silently overwritten.
  saved={...saved,revision:randomUUID(),host:'elders.example.test'};await page.fill('#smtpHost','mijn.example.test');await page.fill('#smtpPassword','NEW-SECRET');await page.click('#smtpSave');await waitFor(()=>document.querySelector('#settingsResult').textContent.includes('elders gewijzigd'));assert.equal(await page.inputValue('#smtpHost'),'mijn.example.test');await page.click('#smtpReload');await waitFor(()=>document.querySelector('#smtpHost').value==='elders.example.test');assert.equal(await page.inputValue('#smtpPassword'),'');
  // Session expiry clears all sensitive drafts and never paints a late response.
  await page.fill('#smtpPassword','DO-NOT-KEEP');expire=true;await page.click('#smtpSave');await page.waitForSelector('#loginPanel:visible');assert.equal(await page.inputValue('#smtpPassword'),'');assert.equal(await page.locator('#settingsPanel').isVisible(),false);assert.deepEqual(errors,[]);
  console.log('PASS: settings route/login, failed-load recovery, save/password preservation, testmail retry ID, stale-write feedback, navigation, light/dark/mobile layout, and expiry clearing');
 }finally{await browser.close();server.close();}
})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
