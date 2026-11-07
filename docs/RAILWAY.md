# LicentieKluis op Railway

1. Maak een service aan vanuit deze repository en kies de branch met de nieuwe versie. Railway leest de Dockerfile en railway.toml.
2. Koppel vóór de eerste deployment een permanent volume aan de service op `/data`. Gebruik één replica.
3. Voor een nieuwe kluis stel je `LKV_ADMIN_USER` en `LKV_ADMIN_PASSWORD` in als servicevariabelen. Gebruik minimaal 14 tekens en één regel voor het wachtwoord. Nadat de eerste start is geslaagd, kun je beide variabelen verwijderen. De aanmeldconfiguratie blijft op het volume bewaard.
4. Bij migratie van Proxmox zet je `secrets.json`, `server.lkv` en eventueel `smtp.lkv` en `mail-log.lkv` samen terug op het volume vóór de eerste start. Maak eerst een back-up, stop de bronserver voor een consistente kopie en gebruik de bij elkaar horende bestanden. De webapp, desktopclients en sync worden daarna op hetzelfde nieuwe HTTPS-adres aangesloten.
5. Controleer de deployment en het antwoord van `/health`. Koppel een Railway-domein als de kluis via internet bereikbaar moet zijn. Test eerst met voorbeeldgegevens, inclusief aanmelden, opnieuw starten en behoud van een proeflicentie.

## Herstelsleutels (server 0.9.0)

Foto’s en tekstbestanden worden opgenomen in de versleutelde `server.lkv` op het bestaande volume. Hiervoor is geen openbare bucket nodig. Behoud het volledige `LKV_DATA`-volume, inclusief `secrets.json`, bij een deployment. Gebruik één replica: de huidige server serialiseert schrijfacties binnen één proces. Bijlagen vallen onder dezelfde back-up en herstelprocedure als licenties. Zie [Herstelsleutels](HERSTELSLEUTELS.md).
