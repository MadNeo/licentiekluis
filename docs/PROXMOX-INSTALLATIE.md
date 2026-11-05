# LicentieKluis installeren op Proxmox

## Nieuwe container: één commando

Open **Proxmox → node → Shell** als root:

```bash
curl -fsSL https://raw.githubusercontent.com/MadNeo/licentiekluis/main/scripts/install-proxmox.sh -o /root/install-proxmox.sh && bash /root/install-proxmox.sh
```

De repository is openbaar; een GitHub-token is niet nodig (Enter bij de tokenvraag). Ontbreekt curl op de host, voer eerst `apt-get update && apt-get install -y curl ca-certificates` uit.

De wizard vraagt opslag, bridge, DHCP of vast IP, eventueel VLAN, servernaam en accounts. Standaard: Ubuntu 24.04, onbevoorrechte LXC, 2 CPU, 2048 MB RAM, 8 GB disk, 512 MB swap en starten bij boot. Ontbrekende hosttools worden via de bestaande APT-bronnen geïnstalleerd. Proxmox moet al aanwezig zijn; Docker, een extra VM en nesting zijn voor deze toepassing niet nodig.

De wizard haalt de actuele broncode van GitHub main op. In de container installeert hij Caddy, .NET 10 SDK/runtime, systeembibliotheken en LicentieKluis. Host en container hebben internettoegang nodig naar GitHub, Ubuntu, Microsoft en NuGet.

## Bestaande container na netwerkfout hervatten

Voor de door deze wizard gemaakte container 102:

```bash
curl -fsSL https://raw.githubusercontent.com/MadNeo/licentiekluis/main/scripts/install-proxmox.sh -o /root/install-proxmox.sh && bash /root/install-proxmox.sh --resume 102
```

Dit behoudt disk, bridge, IP-keuze en VLAN, start de container indien nodig en herstelt de rechten van de bekende systemd-netwerkconfiguratie. De wizard vraagt opnieuw het webaccount en rootwachtwoord en maakt vervolgens de installatie af.

Hervatten vereist een Ubuntu-container met hostname `licentiekluis` en de beschrijving van deze installer. Er mag nog geen software/data onder `/opt/licentiekluis`, `/var/lib/licentiekluis` of Caddy-configuratie bestaan. Bij bestaande of gedeeltelijk geïnstalleerde software stopt hij zonder deze te overschrijven. Gebruik bij een werkende installatie `upgrade`; onderzoek een gedeeltelijke software-installatie voordat je verdergaat. Geen enkele route verwijdert containers of databases.

## Twee verschillende logins

- **LXC-console:** gebruikersnaam `root`. Vul bij de installatie het gewenste rootwachtwoord tweemaal afgeschermd in. Het wordt met `chpasswd` ingesteld en het tijdelijke root-only bestand wordt verwijderd. Het wachtwoord wordt niet in GitHub opgeslagen. Automatisering kan `LKV_ROOT_PASSWORD` aanleveren; zet dit niet in publieke scripts of logs.
- **Webpagina en Windows-serververbinding:** het afzonderlijke LicentieKluis-serveraccount en wachtwoord (minimaal 14 tekens) die je in de wizard kiest.

Op de Proxmox-host kun je ook `pct enter 102` gebruiken. Root-login via SSH wordt niet extra ingeschakeld; gebruik de Proxmox-console voor root met wachtwoord. Het rootwachtwoord van de Proxmox-host zelf wordt niet gewijzigd.

## Webserver en adres

Caddy en LicentieKluis starten automatisch bij boot. Caddy luistert op HTTPS 443 en verwijst door naar de applicatie op localhost 5080. De bestaande webinterface wordt met de server gepubliceerd. HTTP wordt door Caddy naar HTTPS doorgestuurd.

De installatie controleert:

1. Het IPv4-adres van eth0, met een begrensde wachttijd en voortgang.
2. De interne gezondheidscontrole en actieve services.
3. HTTPS met certificaatvalidatie: `/health`, de webpagina `/` en de drie JavaScript/CSS-bestanden.
4. Dezelfde HTTPS-paden vanaf de Proxmox-host via het container-IP.

Pas daarna verschijnt **GEREED: https://...**. Open dit adres in je browser. Dit is het IP van de **container**, niet van de Proxmox-host. Een eigen DNS-naam moet op je pc naar de container verwijzen. Reserveer een DHCP-adres in je router om het adres stabiel te houden.

Bestaande firewallregels blijven behouden. Je pc moet de container op TCP 443 kunnen bereiken (80 voor HTTP-omleiding). De hostcontrole bewijst bereikbaarheid vanaf Proxmox; een aparte pc/VLAN kan aanvullende firewall- of DNS-instellingen nodig hebben.

## Windows: lokaal HTTPS-certificaat vertrouwen

Caddy maakt een lokale CA. De publieke CA wordt op de Proxmox-host opgeslagen als `/root/licentiekluis-CTID-ca.crt`. Vervang `CTID` en `PROXMOX-IP` in PowerShell:

```powershell
scp root@PROXMOX-IP:/root/licentiekluis-CTID-ca.crt "$env:USERPROFILE\Downloads\licentiekluis-ca.crt"
Import-Certificate -FilePath "$env:USERPROFILE\Downloads\licentiekluis-ca.crt" -CertStoreLocation Cert:\CurrentUser\Root
```

Vertrouw alleen de CA van je eigen installatie. Daarna kun je de HTTPS-webpagina openen zonder certificaatwaarschuwing en hetzelfde adres gebruiken in de Windowsapp. Log in met het **serveraccount**, niet automatisch met de LXC-rootlogin.

## Upgrade met behoud van gegevens

Op de Proxmox-host: `pct enter 102`. Vervolgens binnen de container:

```bash
upgrade
```

Dit haalt GitHub main op, bouwt/test, maakt een herstelkopie en behoudt database, accounts, sleutels en SMTP-instellingen. Bij een mislukte start/gezondheidscontrole herstelt de updater de vorige applicatie en gegevens. Data staat in `/var/lib/licentiekluis`, software in `/opt/licentiekluis`. Neem de volledige container met configuratie en sleutels op in Proxmox/PBS-back-ups.

## Verwerkte correcties en diagnose

Bridge-detectie volgt geen sysfs-symlinklussen meer. `pct create` en `pct start` draaien met umask 022 zodat netwerkbestanden leesbaar zijn. Geheimen blijven beschermd met umask 077. De bekende netwerkmap krijgt 755 en eth0.network 644, waarna networkd opnieuw configureert. Pakketinstallatie gebruikt eveneens normale systeemrechten. Een netwerkfout wordt niet stilzwijgend als DHCP-storing aangemerkt.

```bash
pct exec 102 -- ip -4 -br addr
pct exec 102 -- ip route
pct exec 102 -- systemctl status systemd-networkd licentiekluis caddy --no-pager
pct exec 102 -- journalctl -u systemd-networkd -u licentiekluis -u caddy -n 80 --no-pager
```

Bij een fout blijft de container staan. Tijdelijke wachtwoordbestanden worden opgeruimd wanneer de container reageert. Na een abrupte hostonderbreking: verwijder `/root/licentiekluis-init.txt` en `/root/licentiekluis-root.txt` in de container zodra je weer toegang hebt.

Automatische tests simuleren Proxmox, netwerkuitval, HTTPS-uitval, rootwachtwoordoverdracht, hervatten en het weigeren van bestaande data. Deze tests vervangen geen volledige installatie op jouw Proxmox-host.
