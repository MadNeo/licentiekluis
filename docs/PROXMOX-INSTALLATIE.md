# LicentieKluis installeren op Proxmox

## Wat de wizard doet

Start `scripts/install-proxmox.sh` als root in de **Shell van de Proxmox-node**. De wizard maakt zelf een nieuwe Ubuntu 24.04 LXC aan. Standaard: onbevoorrechte container, 2 CPU-kernen, 2048 MB RAM, 8 GB disk, 512 MB swap en automatisch starten na een herstart van Proxmox. Docker, nesting en een extra VM zijn niet nodig.

De wizard controleert de hosttools en installeert ontbrekende curl, CA-certificaten, Python, tar, util-linux en zo nodig het Proxmox-containerpakket via de bestaande APT-bronnen. Proxmox VE zelf moet al zijn geïnstalleerd; de wizard zet een gewone Linuxserver niet om in Proxmox en wijzigt geen Proxmox-repositories.

In de container worden automatisch Caddy (HTTPS), .NET 10 SDK/runtime, benodigde systeembibliotheken, de nieuwste serverbroncode van GitHub `main` en de LicentieKluis-systemd-service geïnstalleerd. Internettoegang vanuit host én container is vereist, onder andere naar GitHub, Ubuntu-pakketbronnen, Microsoft en NuGet.

## Starten

1. Download `install-proxmox.sh` naar je Windows-pc. Het bestand is ook beschikbaar in deze repository onder `scripts/` (bij een privérepository moet je ingelogd zijn).
2. Kopieer het vanuit PowerShell naar de Proxmox-node; vervang `PROXMOX-IP` door het IP-adres van je node:

   ```powershell
   scp "$env:USERPROFILE\Downloads\install-proxmox.sh" root@PROXMOX-IP:/root/install-proxmox.sh
   ```

3. Open **Proxmox → node → Shell** en start:

   ```bash
   bash /root/install-proxmox.sh
   ```

4. Kies container-ID, opslag, bridge, IP-instellingen en eventueel VLAN. Enter gebruikt de getoonde standaard. Een vast IP-adres wordt aanbevolen; bij DHCP moet je een reservering maken. De gateway 192.168.178.1 is een voorstel en moet bij je eigen netwerk passen. DNS wordt overgenomen van de host. Een opgegeven server-DNS-naam moet al naar de container verwijzen.
5. Kies een serveraccount en wachtwoord van minimaal 14 tekens. Dit is de aanmelding voor de server, niet het lokale Windows-kluiswachtwoord. Voor containerbeheer gebruik je `pct enter CTID` op de Proxmox-host; er wordt geen apart rootwachtwoord voor SSH ingesteld.
6. Vul bij een privérepository een GitHub fine-grained token in met toegang tot **MadNeo/licentiekluis**, **Contents: Read-only** (en standaard Metadata). Het token wordt niet getoond en wordt in de container opgeslagen als `/etc/licentiekluis/github-token`, eigenaar root, modus 600, voor toekomstige updates. Gebruik een passende vervaldatum; vervang het token wanneer het verloopt. Bij een openbare repository mag het leeg blijven.
7. Controleer het overzicht en typ `ja`. De wizard downloadt de broncode en de officiële Ubuntu-template, maakt de container en installeert de server. Aan het einde verschijnen het HTTPS-adres en de container-ID.

Een bezet ID wordt geweigerd. Bij een fout wordt geen bestaande VM/LXC verwijderd of gestopt. Een gedeeltelijk geïnstalleerde nieuwe container blijft staan zodat je de fout kunt onderzoeken. Start voor een nieuwe installatie niet opnieuw tegen die gedeeltelijk geïnstalleerde container; controleer hem eerst en kies zo nodig zelf een nieuw vrij ID.

## Windows verbinden

Caddy maakt een eigen lokale CA aan. Aan het einde staat de **publieke** CA op de Proxmox-host als `/root/licentiekluis-CTID-ca.crt`. Kopieer deze vanaf je eigen Proxmox-host naar Windows:

```powershell
scp root@PROXMOX-IP:/root/licentiekluis-CTID-ca.crt "$env:USERPROFILE\Downloads\licentiekluis-ca.crt"
Import-Certificate -FilePath "$env:USERPROFILE\Downloads\licentiekluis-ca.crt" -CertStoreLocation Cert:\CurrentUser\Root
```

Vervang `CTID` en `PROXMOX-IP`. Vertrouw alleen het certificaat afkomstig van jouw eigen installatie. Open daarna het getoonde HTTPS-adres in je browser. Gebruik hetzelfde adres in de Windowsapp bij **Verbinding & back-up** en meld aan met het serveraccount. Windows 0.5.0 synchroniseert daarna automatisch terwijl de kluis geopend is.

Bestaande Proxmox-/netwerkfirewallregels worden niet aangepast. Zorg dat je pc de container op TCP 443 kan bereiken. De interne applicatiepoort 5080 luistert alleen op localhost binnen de container.

## Bijwerken met behoud van gegevens

Op de Proxmox-host:

```bash
pct enter CTID
```

Daarna **binnen de container**:

```bash
upgrade
```

Dit haalt opnieuw GitHub `main` op, bouwt en test de software, maakt een herstelkopie en behoudt licenties, serveraccount, sleutels en SMTP-instellingen. Bij een mislukte start of gezondheidscontrole herstelt het updatescript de vorige applicatie en gegevens. Gebruik de installatiewizard uitsluitend voor nieuwe containers.

De servergegevens staan in `/var/lib/licentiekluis`, los van de software in `/opt/licentiekluis`. Neem de volledige container op in je Proxmox/PBS-back-ups. Bewaar ook de serverconfiguratie en sleutels; alleen het versleutelde gegevensbestand is onvoldoende voor herstel.

## Problemen onderzoeken

```bash
pct enter CTID
systemctl status licentiekluis caddy --no-pager
journalctl -u licentiekluis -u caddy -n 80 --no-pager
curl -fsS http://127.0.0.1:5080/health
```

Bij een fout na aanmaken blijven de container en de uitgepakte broncode beschikbaar. Het tijdelijke bestand met het serverwachtwoord wordt verwijderd als de container bereikbaar is. Bij een abrupt gestopte host of onbereikbare container: verwijder `/root/licentiekluis-init.txt` zodra je weer toegang hebt.

De tests gebruiken geïsoleerde Proxmox-simulaties voor succes, bezet ID, ongeldige repositorytoegang, netwerkuitval en installatiefouten. De echte serverinitialisatie wordt apart getest. Een volledige installatie op jouw Proxmox-host is daarmee niet uitgevoerd.
