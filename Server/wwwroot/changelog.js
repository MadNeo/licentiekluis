'use strict';
(async()=>{const target=document.getElementById('changelogContent');try{const result=await fetch('/changelog.txt',{cache:'no-store'});if(!result.ok)throw Error();target.textContent=await result.text();}catch{target.textContent='De changelog kon niet worden geladen. Vernieuw deze pagina zodra de server bereikbaar is.';}})();
