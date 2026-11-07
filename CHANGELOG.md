# LicentieKluis — wijzigingen

## Windows 0.9.2

- Tijdelijk synchroniseren via HTTPS zonder certificaatcontrole voor lokale IP-adressen en localhost.
- Bestaande lokale verbindingen starten automatisch in deze tijdelijke modus.
- Certificaatcontrole kan per verbinding worden aangezet in Verbinding & back-up. De keuze wordt onthouden.
- Het importeren van een CA-certificaat zet de controle aan. Publieke serveradressen blijven gecontroleerd.

## Windows 0.9.1

- Voeg het publieke CA-certificaat van een lokale Caddy-server toe in Verbinding & back-up.
- Het CA-certificaat geldt alleen voor het opgeslagen serveradres. Aanmelden, synchroniseren, afmelden en mailen gebruiken dezelfde TLS-instellingen.
- Windows meldt een onbekende CA of een afwijkend serveradres in gewone taal.

## Windows 0.9.0

- Vaste plek voor Nieuwe licentie boven de lijst en voor Opslaan rechtsonder in het detailpaneel.
- Aparte onderdelen voor gebruikersbeheer, installatiegegevens en synchronisatiestatus.
- De lijst toont de versie en hoeveel licenties zichtbaar zijn. Zoeken herkent software, versie en apparaatnaam.

## Web 0.7.2

- Eén primaire knop voor het maken van licenties en gegroepeerde beheeracties.
- Zoeken op softwareversie, filters voor geldigheid en een knop om alle filters te wissen.
- Containerconfiguratie voor Railway met een controle op permanente opslag.

## Windows 0.8.0 — 6 november 2026

- Rustigere panelen, afgeronde actieknoppen en duidelijk herkenbare hoofdacties.
- Witte stroken rond tekst in donkere modus verholpen; labels volgen het actuele thema.
- Meer ruimte voor installaties; zoeken doorzoekt nu ook de softwareversie.
- Onopgeslagen licentie-invoer blijft behouden tijdens zoeken en filteren.
- Bewerken blijft mogelijk wanneer de toegewezen gebruiker via de server is verwijderd.
- Een ingevoerd apparaat moet eerst worden toegevoegd voordat de licentie wordt opgeslagen.
- Back-upwachtwoord wordt gewist bij vergrendelen.

## Web 0.7.1 — 6 november 2026

- Vernieuwd dashboard met compacte filters, duidelijke licentiekaarten en een eigen deelblok.
- Nieuw inlogscherm, consistente dialoogvensters en verbeterde mobiele indeling.
- Bijgewerkte lichte en donkere kleuren, toetsenbordfocus en navigatielink naar de inhoud.
- Bestaande beheerfuncties, versleuteling, back-ups en synchronisatie blijven behouden.

## 0.7.0 — 6 november 2026

Windows en web
- Schakelaar voor lichte en donkere modus; de keuze wordt onthouden.
- Leesbare tekst, invoervelden, knoppen en dialoogvensters in beide modi.
- Bestaande licenties, accounts, versleuteling en synchronisatie blijven behouden.

Windows
- Nieuw tabblad Changelog, ook beschikbaar als de kluis vergrendeld is.
- Deze wijzigingenlijst is ingebouwd in de executable en werkt zonder internet.
- Gebaseerd op Windows 0.5.0: behoud van de afzonderlijke versleutelde lokale
  database, opgeslagen verbinding en automatische synchronisatie.

Web
- Changelog-link op het inlogscherm en in de app.
- De changelog wordt door je eigen server geleverd, zonder externe dienst.
- De weergavekeuze blijft behouden na vernieuwen en uitloggen.

## Web/server 0.6 — 6 november 2026

- Gebruikers en licenties toevoegen, bewerken en verwijderen in de browser.
- Versie, einddatum, installatiegegevens en opmerkingen beheren.
- Versleutelde LKV1-back-ups downloaden en herstellen.
- Automatische synchronisatie en expliciete keuzes bij revisieconflicten.
- Versleutelde herstelwachtrij binnen hetzelfde browsertabblad.
- Zoekfilters, codeweergave, kopiëren en gecontroleerde e-mailverzending behouden.
- Ontbrekend upgrade-commando herstellen; database en instellingen behouden.

## Windows 0.5.0 — 5 november 2026

- Versleutelde lokale SQLite-database buiten de executable.
- Automatische synchronisatie iedere 30 seconden bij beschikbare verbinding.
- Lokaal doorwerken bij een onbereikbare server.
- Opgeslagen serververbinding versleuteld bewaren.

## Windows 0.4.1 — 5 november 2026

- Correcties voor zichtbaarheid van tekst, bediening en vensterindeling.

## Windows 0.4.0

- Vernieuwde indeling van het hoofdvenster en aparte vensters voor verbinding,
  back-up en het delen van licenties.
