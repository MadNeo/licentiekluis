# LicentieKluis

Native Rust Windowsclient en .NET 10-server met webportaal voor licenties per gebruiker, platform en apparaat. Offline opslag, synchronisatie, conflictafhandeling en meerdere licentiecodes per e-mail. Dit pakket bevat de broncode voor Windows 0.9.1 en web/server 0.7.2.

## Server installeren in Proxmox-LXC

Open de shell van je **Proxmox-host** als root en voer uit:

```bash
curl -fsSL https://raw.githubusercontent.com/MadNeo/licentiekluis/main/scripts/install-proxmox.sh -o /root/install-proxmox.sh && bash /root/install-proxmox.sh
```

De wizard maakt een Ubuntu 24.04-container en installeert Caddy, .NET 10 en de actuele LicentieKluis-broncode van main. De repository is openbaar; laat de tokenvraag leeg. Zie [de Proxmox-installatiehandleiding](docs/PROXMOX-INSTALLATIE.md) voor netwerkkeuzes, hervatten, accounts en certificaten.

Heb je al een geschikte lege container, dan kun je daar als root `scripts/install-lxc.sh` uitvoeren. Gebruik bij een bestaande LicentieKluis-installatie altijd `upgrade`.

Open daarna `https://<IP-of-DNS-naam>/` en vertrouw het Caddy-CA-certificaat op je apparaat. Het webportaal ondersteunt gebruikers- en licentiebeheer, installatiegegevens, filters, codeweergave, e-mail, versleutelde back-up/herstel en revisieconflicten. Zie [de webhandleiding](docs/WEB-BEHEER.md). Bestaande gegevens en synchronisatie met Windows blijven behouden. Voor zelfstandig offline gebruik blijft de Windows-app beschikbaar.

## Webapp draaien op Railway

Zie [Railway-installatie](docs/RAILWAY.md) voor de container, blijvende opslag en de eerste aanmelding.

## Bijwerken

Als root in de LXC:

```bash
upgrade
```

Dit haalt de nieuwste commit op main op, bouwt en test die, stopt daarna kort de server, maakt een back-up en vervangt de programmabestanden. De map `/var/lib/licentiekluis` blijft behouden, inclusief `server.lkv`, `secrets.json`, SMTP-instellingen en maillog. Ook de systemd- en Caddy-configuratie blijven behouden. Het script voert bij een upgrade nooit `--init` uit.

Back-ups staan in `/opt/licentiekluis/backup-<datum>-<tijd>.<uniek>/`. Bij een startfout worden de vorige programmaversie en de gegevenssnapshot teruggezet. Controleer bij fouten `journalctl -u licentiekluis`; een mislukte gegevensherstelactie laat de server gestopt. Bewaar deze back-ups beveiligd en kopieer ze ook buiten de container. Een stroomstoring of harde onderbreking kan handmatig herstel vereisen.

De scripts werken de .NET 10 SDK/runtime eveneens bij; een runtime-update wordt bij applicatieherstel niet teruggedraaid. `main` betekent de nieuwste broncode, niet noodzakelijk een versie waarvan GitHub Actions al geslaagd is. Test de preview eerst met voorbeeldgegevens.

### Bestaande installatie van 0.1 / 0.2 / 0.3 overzetten

Kopieer `scripts/install-lxc.sh` vanuit GitHub naar de bestaande LXC en voer eenmaal uit:

```bash
bash /root/install-lxc.sh upgrade
```

Daarna is het commando `upgrade` beschikbaar. De bestaande gegevens en accounts blijven behouden. Dit ondersteunt de standaardpaden van de eerdere installer.

### Token optioneel bewaren

Bij iedere upgrade kan het token opnieuw worden ingevoerd. Voor gebruik zonder invoer kun je het optioneel root-only opslaan:

```bash
install -d -m 700 /etc/licentiekluis
read -rsp 'GitHub-token: ' token; echo
(umask 077; printf '%s' "$token" > /etc/licentiekluis/github-token)
chmod 600 /etc/licentiekluis/github-token
unset token
```

Ook `GITHUB_TOKEN` als omgevingsvariabele wordt ondersteund. Tokens worden nooit in de repository of back-ups van de database opgenomen.

## Windowsclient

Installeer Rust en Visual Studio Build Tools met Desktop development with C++. Start `Build-Rust.cmd`. De uitvoer staat in `output/Windows-Rust/LicentieKluis-Rust.exe`. De client heeft geen .NET of WebView2 nodig. De GitHub Actions-workflow bouwt Windows en server en biedt downloadbare build-artifacts aan wanneer de controles slagen.

## Teststatus

Zie GitHub Actions voor de actuele buildresultaten. Een volledige installatie en upgrade in een echte Proxmox-LXC en echte SMTP-verzending moeten nog praktisch worden gecontroleerd. De oorspronkelijke uitgebreide gebruiksinstructies staan in `README.html`; voor installatie en updates heeft deze README voorrang.

## PDF-handleidingen

- [Handleiding serversoftware](docs/LicentieKluis_Server_Handleiding.pdf)
- [Handleiding Windowssoftware](docs/LicentieKluis_Windows_Handleiding.pdf)

Dit zijn de eerder opgeleverde PDF-handleidingen. Voor de daarna toegevoegde GitHub-installatie en het commando `upgrade` volg je de actuele instructies hierboven.


## Weergave en wijzigingen

[Lichte/donkere modus](docs/WEERGAVE.md) · [Changelog van deze versie](CHANGELOG.md). De Windowsclient op main bevat ook de versleutelde lokale SQLite-opslag en automatische synchronisatie uit Windows 0.5.0.
