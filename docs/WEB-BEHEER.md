# LicentieKluis web 0.6

De webversie beheert dezelfde servergegevens en gebruikt hetzelfde revisieprotocol als de Windows-app. De bestaande database, accounts, encryptiesleutels en SMTP-instellingen blijven ongewijzigd.

## Bestaande installatie bijwerken

Voer als root **in de LicentieKluis-container** uit:

```bash
upgrade
```

De updater maakt een back-up en behoudt de gegevens. Open daarna het HTTPS-adres van je container en vernieuw met Ctrl+F5.

## Beheer

- **Gebruikers beheren**: toevoegen, hernoemen en verwijderen. Dit zijn licentiehouders, geen inlogaccounts. Verwijderde gebruikers blijven intern als historische verwijzing bestaan. Hun licenties blijven zichtbaar en kunnen aan een andere gebruiker worden toegewezen.
- **Nieuwe licentie / Bewerken**: software, gebruiker, licentiecode, versie, einddatum, opmerkingen en maximaal 100 installaties. Per installatie kies je Windows, Linux of macOS en een apparaatnaam.
- **Verwijderen**: vraagt bevestiging en bewaart intern een verwijdermarkering voor de synchronisatie.
- Zoeken op software, gebruiker of apparaat; filters op gebruiker en platform; tijdelijk tonen en kopiëren van codes blijven beschikbaar.
- E-mailselectie, controle vooraf en expliciet versturen blijven beschikbaar. Synchroniseer wachtende wijzigingen eerst. Er wordt nooit automatisch een e-mail verstuurd.

## Synchronisatie en conflicten

Wijzigingen worden direct aangeboden aan de server. Iedere 30 seconden en bij terugkeer van de verbinding controleert de pagina opnieuw. De Windows-app ontvangt de wijzigingen bij zijn volgende synchronisatie.

Gelijktijdige wijzigingen worden niet stilzwijgend overschreven. **Conflicten** toont beide versies en biedt de keuze **Mijn wijziging behouden** of **Serverversie behouden**. Ook een bewerkingsformulier dat tijdens een serverwijziging openstond houdt zijn oorspronkelijke revisie vast.

## Verbinding verbroken

De geopende pagina kan verder werken met de geladen gegevens. Wachtende wijzigingen worden met AES-256-GCM en een van het serverwachtwoord afgeleide sleutel versleuteld in `sessionStorage` opgeslagen. Na opnieuw inloggen in hetzelfde tabblad wordt de wachtrij hervat. Wachtwoorden en sessietokens worden niet in browseropslag bewaard. Bij geblokkeerde/volle browseropslag verschijnt een fout; houd het tabblad open en download een back-up.

Dit is **geen zelfstandige offline Windows-database**. Het sluiten van het tabblad kan de herstelwachtrij verwijderen; maak bij een storing eerst een back-up. Voor langdurig offline werken, ook na het opnieuw starten van het programma, gebruik je de Windows-app. De webpagina vergrendelt na 15 minuten inactiviteit. Bij een gewijzigd serverwachtwoord kan een bestaande wachtrij nog met het oude wachtwoord versleuteld zijn; download dan de herstelback-up via het inlogscherm en importeer deze met het oorspronkelijke wachtwoord.

## Back-up en herstellen

1. Open **Back-up en herstellen**.
2. Kies voor een nieuwe back-up een wachtwoord van minimaal 14 tekens en herhaal het.
3. Download het `.lkv`-bestand. Dit bevat ook wachtende wijzigingen en verwijdermarkeringen. Bewaar het wachtwoord apart.
4. Voor herstellen selecteer je een `.lkv`-bestand, vul je het bijbehorende wachtwoord in en kies je **Back-up controleren**.
5. Controleer de aantallen en maak eerst een back-up van de huidige kluis. Kies vervolgens **Gecontroleerde back-up herstellen** en bevestig.

LKV1 gebruikt PBKDF2-SHA256 (600.000 iteraties) en AES-256-GCM, zoals Windows. Zowel camelCase- als PascalCase-records worden gelezen. Een verkeerd wachtwoord of aangepast bestand wordt afgewezen.

Herstellen is samenvoegen: records met hetzelfde ID worden vervangen, andere records blijven bestaan. Verwijdermarkeringen uit de back-up worden meegenomen. De serververbinding en inloggegevens worden niet uit een back-up overgenomen. Wijzigingen na het controleren van de back-up worden als conflicten behandeld.

## Validatie

- Bestaande web- en e-mailregressietests.
- Protocoltests voor wijzigingen tijdens synchronisatie, revisieconflicten en behoud van licenties bij verwijderde gebruikers.
- Back-upuitwisseling met een onafhankelijke implementatie van het Windows/.NET LKV1-formaat, verkeerde wachtwoorden en beschadigde bestanden.
- Gepubliceerde server: routes, authenticatie, synchronisatie en versleutelde gegevens na herstart.
- Chromium met synthetische API: gebruikers/licenties beheren, conflicten, back-up/herstel, e-mailcontrole, verbindingsverlies, opnieuw inloggen, mobiel en vergrendeling. Geen echte e-mailverzending.

## Fout: upgrade: command not found

Het commando ontbreekt of staat niet in het zoekpad van de console. Herstel en start de update als root **binnen de bestaande LicentieKluis-container**:

```bash
curl -fsSL https://raw.githubusercontent.com/MadNeo/licentiekluis/main/scripts/install-lxc.sh -o /root/licentiekluis-upgrade.sh && bash /root/licentiekluis-upgrade.sh upgrade
```

Laat de GitHub-tokenvraag leeg voor de openbare repository. De procedure registreert eerst het commando en begint daarna aan de update. Database en instellingen blijven behouden. Dit is een upgrade, geen nieuwe installatie. Een bestaande LicentieKluis-service is vereist.

Controle achteraf: `upgrade --help`. Voortaan volstaat `upgrade`. De registratie beschermt andere bestaande commando's met dezelfde naam en biedt ook toegang via `/usr/bin/upgrade` als `/usr/local/bin` ontbreekt in PATH.

## Instellingen (web 0.8.0)

Open **Instellingen** in de bovenste navigatie. De pagina bevat E-mail, Weergave, Back-up en herstel, en Serverinformatie. Je blijft aangemeld bij het wisselen tussen licenties en instellingen. Een directe link naar `/settings` vraagt eerst om je serveraccount.

### E-mail instellen zonder scripts

1. Vul SMTP-host, STARTTLS-poort (meestal 587) en afzenderadres in.
2. Vul de SMTP-gebruikersnaam en het wachtwoord of app-wachtwoord in. Schakel aanmelding alleen uit als je een vertrouwde relay gebruikt. Poort 465 en OAuth worden nog niet ondersteund.
3. Kies **Instellingen opslaan**. De server gebruikt deze direct, ook voor e-mail vanuit Windows. Er is geen herstart nodig.
4. Vul je eigen adres in onder **Controleer met een testmail** en kies **Testmail versturen**. Controleer inbox en ongewenste mail. Er worden geen licenties meegestuurd.

Een opgeslagen wachtwoord wordt nooit teruggestuurd naar de browser. Laat het wachtwoordveld leeg om het te behouden. Bij wijziging van host, poort of gebruikersnaam moet je het wachtwoord opnieuw invoeren. Gelijktijdige wijzigingen worden gemeld, zodat je geen instellingen van een andere sessie overschrijft.

Bij een onzekere testmailstatus controleer je eerst je inbox. De knop **Status opnieuw controleren** hergebruikt hetzelfde verzoeknummer en voorkomt opnieuw versturen als de server het bericht al heeft aangeboden. Een nieuw testmailverzoek verstuurt bewust een nieuw bericht.

Bestaande `smtp.lkv`-instellingen worden automatisch gelezen en blijven versleuteld opgeslagen. Het oude configuratiescript blijft beschikbaar voor bestaande installaties, maar is niet nodig voor instellingenbeheer. Na een eenmalige update naar web 0.8.0 regel je dit via de pagina.
