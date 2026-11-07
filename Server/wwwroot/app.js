"use strict";
const $ = id => document.getElementById(id);
let token = "", entries = [], generation = 0, idle;
const timers = new Set();
function node(tag, text, cls) { const n=document.createElement(tag); if(text!==undefined)n.textContent=text; if(cls)n.className=cls; return n; }
function message(s=""){ $("message").textContent=s; }
function localLock(){ if(typeof manageLock==="function")manageLock(); if(typeof resetMail==="function")resetMail(); generation++; token=""; entries=[]; for(const t of timers)clearTimeout(t);timers.clear();clearTimeout(idle); $("licenses").replaceChildren();$("owner").replaceChildren(new Option("Alle gebruikers",""));resetFilters();$("count").textContent="";$("dashboard").hidden=true;$("logout").hidden=true;$("loginPanel").hidden=false;$("password").value=""; }
async function logout(){ if(typeof vault!=="undefined"&&vault?.pending.size){try{await persistPending(vault,generation);}catch{message("Wachtende wijzigingen konden niet veilig worden bewaard. Maak eerst een versleutelde back-up.");return;}}const old=token; localLock(); if(old)try{await fetch("/api/logout",{method:"POST",headers:{Authorization:`Bearer ${old}`},cache:"no-store"});}catch{} }
function activity(){clearTimeout(idle);if(token)idle=setTimeout(()=>{logout();message("Je bent uitgelogd wegens inactiviteit.");},15*60*1000);}
for(const evt of ["pointerdown","keydown"])document.addEventListener(evt,activity,{passive:true});
async function api(path){const current=token;const r=await fetch(path,{headers:{Authorization:`Bearer ${current}`},cache:"no-store"});if(current!==token)throw new Error("Sessie gewijzigd.");if(r.status===401){localLock();throw new Error("Je sessie is verlopen. Log opnieuw in.");}if(!r.ok)throw new Error("Ophalen mislukt. Controleer de serververbinding.");return r.json();}
async function refresh(){if(typeof manageRefresh==="function"&&vault)return manageRefresh();const g=generation;message();$("refresh").disabled=true;try{const data=await api("/api/entries");if(g!==generation)return;entries=data;const selected=$("owner").value;$("owner").replaceChildren(new Option("Alle gebruikers",""));entries.filter(x=>x.kind==="person").sort((a,b)=>a.name.localeCompare(b.name)).forEach(p=>$("owner").add(new Option(p.name,p.id)));$("owner").value=selected;render();}catch(e){if(g===generation)message(e.message);}finally{$("refresh").disabled=false;}}
function resetFilters() {
  for (const id of ["search", "owner", "platform", "expiryFilter"]) $(id).value = "";
  $("clearFilters").disabled = true;
}
function dateKey(date) {
  return [date.getFullYear(), date.getMonth() + 1, date.getDate()]
    .map((part, index) => String(part).padStart(index === 0 ? 4 : 2, "0")).join("-");
}
function render() {
  if (typeof mailBeginRender === "function") mailBeginRender();
  for (const t of timers) clearTimeout(t);
  timers.clear();
  const list = $("licenses");
  list.replaceChildren();
  const people = new Map(entries.filter(entry => entry.kind === "person").map(person => [person.id, person.name]));
  const query = $("search").value.trim().toLocaleLowerCase();
  const owner = $("owner").value;
  const platform = $("platform").value;
  const expiry = $("expiryFilter").value;
  const filtered = Boolean(query || owner || platform || expiry);
  $("clearFilters").disabled = !filtered;
  const today = new Date();
  const horizon = new Date(today);
  horizon.setDate(horizon.getDate() + 30);
  const from = dateKey(today);
  const to = dateKey(horizon);
  const all = entries.filter(entry => entry.kind === "license");
  const licenses = all.filter(entry => {
    if (owner && entry.ownerId !== owner) return false;
    if (platform && !entry.installations.some(item => item.platform === platform)) return false;
    if (expiry === "expired" && (!entry.expires || entry.expires >= from)) return false;
    if (expiry === "soon" && (!entry.expires || entry.expires < from || entry.expires > to)) return false;
    if (expiry === "none" && entry.expires) return false;
    return [entry.name, entry.softwareVersion, people.get(entry.ownerId), ...entry.installations.map(item => item.device)]
      .join(" ").toLocaleLowerCase().includes(query);
  }).sort((a, b) => a.name.localeCompare(b.name, "nl"));
  $("count").textContent = filtered
    ? `${licenses.length} van ${all.length} licenties zichtbaar`
    : `${all.length} ${all.length === 1 ? "licentie" : "licenties"} in de kluis`;
for(const e of licenses){const card=node("article",undefined,"license");card.append(node("h3",e.name),node("p",people.get(e.ownerId)||"Verwijderde gebruiker","owner"));if(e.softwareVersion)card.append(node("span",`Versie ${e.softwareVersion}`,"badge"));for(const p of new Set(e.installations.map(i=>i.platform)))card.append(node("span",p,"badge"));const expiryLabel = e.expires && (e.expires < from ? "Verlopen" : e.expires <= to ? "Verloopt binnenkort" : "Geldig tot");
card.append(node("p", e.expires ? `${expiryLabel} · ${e.expires.split("-").reverse().join("-")}` : "Geen einddatum", e.expires && e.expires < from ? "expired" : e.expires && e.expires <= to ? "expiring" : "muted"));
for(const i of e.installations)card.append(node("div",`${i.platform} · ${i.device||"Geen apparaatnaam"}`,"install"));if(e.notes)card.append(node("p",e.notes,"notes"));const code=node("code","•••• •••• ••••","code");card.append(code);const actions=node("div",undefined,"actions"),show=node("button","Code tonen"),copy=node("button","Kopiëren");
const fetchCode=async()=>{const g=generation;const data=typeof vault!=="undefined"&&vault?{code:vault.records.get(e.id)?.code||""}:await api(`/api/licenses/${encodeURIComponent(e.id)}/code`);if(g!==generation||!card.isConnected)throw new Error("Overzicht gewijzigd.");return data.code;};
show.addEventListener("click",async()=>{show.disabled=true;try{code.textContent=await fetchCode();const t=setTimeout(()=>{code.textContent="•••• •••• ••••";timers.delete(t);},30000);timers.add(t);}catch(err){message(err.message);}finally{show.disabled=false;}});
copy.addEventListener("click",async()=>{copy.disabled=true;try{const value=await fetchCode();await navigator.clipboard.writeText(value);copy.textContent="Gekopieerd";const t=setTimeout(()=>{copy.textContent="Kopiëren";timers.delete(t);},2000);timers.add(t);}catch{message("Kopiëren niet toegestaan door de browser. Gebruik Code tonen en kopieer handmatig.");}finally{copy.disabled=false;}});actions.append(show,copy);card.append(actions);if(typeof manageControls==="function")manageControls(card,e);if(typeof mailControls==="function")mailControls(card,e);list.append(card);}
if(typeof mailEndRender==="function")mailEndRender();if (!licenses.length) {
  list.append(node("div", filtered
    ? "Geen resultaten. Pas je zoekterm aan of wis de filters."
    : "Je kluis is leeg. Kies Nieuwe licentie om te beginnen.", "empty"));
}}
$("loginForm").addEventListener("submit",async ev=>{ev.preventDefault();const g=generation;message();$("submit").disabled=true;try{if(location.protocol!=="https:"&&!['localhost','127.0.0.1'].includes(location.hostname))throw new Error("Gebruik het HTTPS-adres van de server.");const r=await fetch("/api/login",{method:"POST",headers:{"Content-Type":"application/json"},cache:"no-store",body:JSON.stringify({username:$("username").value,password:$("password").value})});if(!r.ok)throw new Error(r.status===429?"Te veel pogingen. Wacht een minuut.":"Aanmelden mislukt. Controleer je account en wachtwoord.");const result=await r.json();if(g!==generation)return;token=result.token;if(typeof manageLogin==="function")await manageLogin(result,$("password").value);if(g!==generation)return;$("loginPanel").hidden=true;$("dashboard").hidden=false;$("logout").hidden=false;activity();await refresh();}catch(e){if($("dashboard").hidden)token="";message(e.message);}finally{$("password").value="";$("submit").disabled=false;}});
$("logout").addEventListener("click",()=>{if(typeof vault!=="undefined"&&vault?.pending.size&&!confirm("Er zijn niet gesynchroniseerde wijzigingen. Maak eerst een back-up. Toch uitloggen?"))return;logout();message();});$("refresh").addEventListener("click",refresh);$("owner").addEventListener("change",render);$("platform").addEventListener("change",render);$("search").addEventListener("input",render);window.addEventListener("pagehide",localLock);window.addEventListener("pageshow",e=>{if(e.persisted)localLock();});


$("expiryFilter").addEventListener("change", render);
$("clearFilters").addEventListener("click", () => { resetFilters(); render(); });
