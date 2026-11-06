# Windows 0.5.0: lokale database en automatische synchronisatie

## Opslag los van de exe

De Windowsapp bewaart gegevens in `%LOCALAPPDATA%\LicentieKluisRust\vault.db`.
De exe mag worden vervangen of vanuit een andere map worden gestart; dezelfde Windowsgebruiker blijft dezelfde database gebruiken. De database blijft bestaan als de exe wordt verwijderd. Verwijder deze gegevensmap niet.

SQLite bevat uitsluitend versleutelde gegevensblokken (AES-256-GCM, sleutel afgeleid van het kluiswachtwoord met PBKDF2-SHA256 en 600.000 iteraties). Ook de opgeslagen serveraanmelding is versleuteld. De SQLite-structuur zelf is zichtbaar, maar namen, licentiecodes en serverwachtwoorden staan niet als leesbare tekst in de database. Het kluiswachtwoord wordt niet opgeslagen.

Bij het eerste ontgrendelen wordt een bestaande `vault.lkv` automatisch overgenomen, indien de database nog geen kluis bevat. Het oorspronkelijke bestand blijft als migratiekopie bewaard; nieuwe wijzigingen worden alleen in de database opgeslagen.

## Lokaal doorwerken

Ontgrendel met je bestaande kluiswachtwoord. Lezen, toevoegen, wijzigen en verwijderen werken zonder server. Opslaan schrijft eerst naar de lokale database. Wijzigingen die nog naar de server moeten, blijven bewaard na afsluiten en opnieuw starten.

## Server instellen

1. Open **Verbinding & back-up**.
2. Vul het HTTPS-serveradres, serveraccount en serverwachtwoord in.
3. Klik op **Aanmelden** en bevestig het koppelen van een nieuwe kluis.
4. Deze aanmelding wordt versleuteld bewaard voor automatische herverbinding.

Na ontgrendelen probeert de app direct te synchroniseren, daarna iedere 30 seconden na afronding van de vorige poging. Bij een onbereikbare server blijft de lokale database bruikbaar en volgt automatisch een nieuwe poging. Er verschijnen geen herhaalde foutvensters. De statusbalk toont een fout en het opnieuw proberen. Bij verkeerde accountgegevens moet je deze via Aanmelden corrigeren.

Synchronisatie draait op de achtergrond. Lokale wijzigingen tijdens een netwerkverzoek blijven behouden; niet-opgeslagen invoer wordt niet door een netwerkantwoord gewist. Conflicten worden niet stil overschreven: gebruik **Conflicten** om te kiezen welke versie behouden blijft.

**Afmelden** verwijdert de bewaarde serveraanmelding en stopt automatische synchronisatie. **Vergrendelen** stopt synchronisatie zolang de kluis dicht is; opnieuw ontgrendelen hervat deze. Bij **Herstellen** van een back-up moet de server opnieuw worden aangemeld.

## Back-ups en bereikbaarheid

Gebruik **Back-up maken** voor een versleutelde `.lkv`-export en bewaar die buiten deze computer. Het exportbestand bevat licenties en synchronisatiegegevens, geen opgeslagen serverwachtwoord. Bewaar ook je kluiswachtwoord: zonder dit wachtwoord kunnen de gegevens niet worden ontsleuteld.

De lokale database hoort bij deze Windowsgebruiker en computer. Voor een andere computer gebruik je de server of een versleutelde back-up. Dit is geen Windows-achtergrondservice: automatische synchronisatie werkt zolang de app geopend en de kluis ontgrendeld is.

