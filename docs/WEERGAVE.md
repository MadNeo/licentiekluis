# Lichte en donkere modus — versie 0.7.0

## Windows

Bovenaan staan de tabbladen **Kluis** en **Changelog** en de knop **Donkere modus** of **Lichte modus**. De keuze geldt ook voor Verbinding & back-up en Licenties delen. Omschakelen verandert geen licentiegegevens of geopende invoer.

De voorkeur staat in `%LOCALAPPDATA%\LicentieKluisRust\appearance.txt`, los van de versleutelde database. Een nieuwe executable behoudt daardoor zowel de database als de weergavekeuze. De changelog is ingebouwd en kan ook zonder serververbinding of ontgrendelde kluis worden gelezen.

Windows gebruikt zijn eigen systeemvensters voor bestandskeuze en meldingen. Deze volgen waar van toepassing de Windows-instellingen.

## Web

De knop staat bovenaan, ook vóór het inloggen. De browser onthoudt alleen de weergavekeuze in lokale opslag. Zonder opgeslagen keuze volgt de webapp de systeemvoorkeur bij openen. Bij geblokkeerde browseropslag werkt de schakelaar nog steeds, maar wordt de keuze niet blijvend bewaard.

De link **Changelog** opent de wijzigingen van de geïnstalleerde serverversie. Deze pagina gebruikt dezelfde weergavekeuze. De inhoud komt uit de meegeleverde CHANGELOG.md en niet uit een externe website.

Werk een bestaande server als root in de LXC bij met `upgrade` en vernieuw de browser met Ctrl+F5. Gebruik de nieuwe Windows-executable voor het tabblad en de schakelaar in Windows.
