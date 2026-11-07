# Herstelsleutels — Windows 1.0.0 / web en server 0.9.0

Open **Herstelsleutels** in de Windowsapp of de menubalk van het webportaal. Kies **Nieuwe herstelsleutel**, geef een herkenbare naam op en kies **Authenticator** of **Crypto-wallet**.

Vul minimaal één mogelijkheid in. Je mag ze ook combineren:

| Invoer | Ondersteuning |
| --- | --- |
| Handmatig ingevoerde tekst | Herstelcodes of herstelzin, maximaal 20.000 tekens. Volgorde en regeleinden blijven behouden. |
| Tekstbestand | Niet-leeg `.txt` in UTF-8, maximaal 64 KB. |
| Foto | PNG of JPEG, maximaal 2 MB per foto. |

Een item bevat maximaal één tekstbestand en één foto. Een nieuwe bijlage vervangt de vorige; **wissen/verwijderen** verwijdert alleen die invoervorm. Een naam of categorie wijzigen behoudt de overige inhoud. De herstelinhoud van alle items samen is begrensd op 8 MB, gerekend inclusief de base64-opslag van foto's. Kies kleinere foto's als deze grens wordt bereikt.

**Bekijken** opent tekst en foto in het programma. Na 30 seconden sluit het voorbeeld. De Windowsapp schrijft hiervoor geen tijdelijke foto naar schijf. In het webportaal wordt het voorbeeld ook gesloten wanneer het tabblad naar de achtergrond gaat. Bestaande handmatige tekst is bij bewerken eerst verborgen; klik op **Tekst tonen / bewerken**. Herstelsleutels kunnen niet via de functie **Licenties delen** worden verstuurd.

## Opslaan, synchroniseren en herstellen

Windows bewaart herstelgegevens in dezelfde versleutelde lokale SQLite-kluis als licenties. In het webportaal wordt na opslaan meteen gesynchroniseerd; Windows hervat automatische synchronisatie wanneer je het venster Herstelsleutels sluit. Wachtende wijzigingen zijn herkenbaar in het overzicht. Je kunt Windows zonder server gebruiken.

Alle invoervormen en bijlagen gaan mee in `.lkv`-back-ups en in revisieconflicten. Maak en herstel back-ups via **Instellingen → Back-up en herstel**. Verwijderen maakt een synchroniseerbare verwijdermarkering zonder herstelinhoud; eerder gemaakte back-ups kunnen de oorspronkelijke inhoud nog bevatten.

Bij gelijktijdige wijzigingen wordt niets stilzwijgend overschreven. De webeditor weigert een inmiddels verouderd formulier; de synchronisatie bewaart conflicten tot je een versie kiest. Conflictvoorbeelden schermen de inhoud af.

Houd bij een internetstoring het browsertabblad open. Wachtende wijzigingen worden versleuteld in de tabbladopslag bewaard. Bij veel/grote foto's kan de browserquota bereikt worden: synchronisatie wordt dan nog geprobeerd; maak bij een storing direct een versleutelde back-up. De Windowsapp heeft deze browserbeperking niet.

## Bijwerken en compatibiliteit

Werk eerst de server bij naar 0.9.0 en vervolgens alle Windowsclients naar 1.0.0. Vernieuw geopende webpagina's. Bestaande licenties en oudere back-ups blijven leesbaar in de nieuwe versies. Zodra een kluis herstelsleutels bevat, blokkeert de server oudere synchronisatieclients met een updatemelding. Oude software begrijpt de nieuwe records niet; gebruik voor terugdraaien een back-up van vóór de update.

De gegevens zijn versleuteld opgeslagen, maar dit is geen end-to-end-versleutelde kluis: het aangemelde serveraccount en de server kunnen de inhoud ontsleutelen. Gebruik een vertrouwde server en een sterk kluiswachtwoord. De bestaande tijdelijke instelling voor lokale certificaatcontrole blijft van toepassing; openbare HTTPS-adressen blijven certificaten controleren. Bewaar een onafhankelijke herstelkopie zodat toegang tot deze kluis niet afhankelijk is van de enige daarin opgeslagen herstelcode.

## Ontwerp

[Figma: geïntegreerde herstelsleutels](https://www.figma.com/design/ZU6spcSPPACjX5n4QE5N1N)

Het onderdeel gebruikt dezelfde kleuren, knoppen en lichte/donkere modus als de licentie- en instellingenpagina's. Het overzicht toont alleen naam, categorie en aanwezige invoervormen.
