'use strict';
(function(){
 const key='licentiekluis-theme';let chosen;
 try{chosen=localStorage.getItem(key);}catch{}
 let dark=chosen==='dark'||(chosen!=='light'&&window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches);
 function apply(){document.documentElement.dataset.theme=dark?'dark':'light';const button=document.getElementById('themeToggle');if(button){button.textContent=dark?'Lichte modus':'Donkere modus';button.setAttribute('aria-pressed',String(dark));button.setAttribute('aria-label',dark?'Schakel naar lichte modus':'Schakel naar donkere modus');}}
 apply();document.addEventListener('DOMContentLoaded',()=>{apply();document.getElementById('themeToggle')?.addEventListener('click',()=>{dark=!dark;try{localStorage.setItem(key,dark?'dark':'light');}catch{}apply();});});
 window.addEventListener('storage',event=>{if(event.key===key&&['dark','light'].includes(event.newValue)){dark=event.newValue==='dark';apply();}});
})();
