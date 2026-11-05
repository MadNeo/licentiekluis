#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
[[ $EUID -eq 0 ]] || { echo 'Voer uit als root BINNEN de LXC, niet op de Proxmox-host.'; exit 1; }
source /etc/os-release
[[ "$ID" == ubuntu && "$VERSION_ID" == 24.04 ]] || { echo 'Deze installer ondersteunt Ubuntu 24.04 LTS.'; exit 1; }
[[ $(uname -m) == x86_64 ]] || { echo 'Deze preview ondersteunt x86_64.'; exit 1; }
[[ ! -e /opt/licentiekluis && ! -e /var/lib/licentiekluis ]] || { echo 'Bestaande installatie gevonden. Gebruik upgrade; data niet overschreven.'; exit 1; }
[[ ! -e /etc/caddy/Caddyfile ]] || { echo 'Gebruik een lege, aparte LXC; er bestaat al een Caddy-configuratie.'; exit 1; }
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
server_name=${LKV_SERVER_NAME:-}
if [[ -z "$server_name" ]]; then read -rp 'Vast IP-adres of lokale DNS-naam van deze LXC (zonder https://): ' server_name; fi
init_file=${LKV_INIT_FILE:-}
if [[ -n "$init_file" ]]; then
    [[ -f "$init_file" && $(stat -c '%u:%a' "$init_file") == 0:600 ]] || { echo 'Initialisatiebestand moet root en chmod 600 hebben.'; exit 1; }
    trap 'rm -f -- "$init_file"' EXIT
fi
[[ "$server_name" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*[a-zA-Z0-9]$ ]] || { echo 'Ongeldige hostnaam / IPv4-adres.'; exit 1; }
(umask 022; apt-get update; apt-get install -y ca-certificates curl caddy libicu74 libssl3t64 zlib1g)
useradd --system --home /var/lib/licentiekluis --shell /usr/sbin/nologin licentiekluis
install -d -m 755 /opt/licentiekluis /opt/licentiekluis/dotnet /opt/licentiekluis/app
install -d -m 700 /var/lib/licentiekluis
curl --fail --show-error --location --proto '=https' --tlsv1.2 https://dot.net/v1/dotnet-install.sh -o /opt/licentiekluis/dotnet-install.sh
bash /opt/licentiekluis/dotnet-install.sh --channel 10.0 --install-dir /opt/licentiekluis/dotnet --no-path
chmod -R a+rX /opt/licentiekluis/dotnet
export DOTNET_ROOT=/opt/licentiekluis/dotnet
/opt/licentiekluis/dotnet/dotnet run --project "$source_dir/Tests/LicenseVault.Tests.csproj" -c Release
/opt/licentiekluis/dotnet/dotnet publish "$source_dir/Server/LicenseVault.Server.csproj" -c Release --self-contained false -p:UseAppHost=false -o /opt/licentiekluis/app
cp "$source_dir/GITHUB-REVISION" /opt/licentiekluis/app/GITHUB-REVISION
chmod -R a+rX /opt/licentiekluis/app
if [[ -n "$init_file" ]]; then
    LKV_DATA=/var/lib/licentiekluis /opt/licentiekluis/dotnet/dotnet /opt/licentiekluis/app/LicenseVault.Server.dll --init-stdin < "$init_file"
    rm -f -- "$init_file"
else
    LKV_DATA=/var/lib/licentiekluis /opt/licentiekluis/dotnet/dotnet /opt/licentiekluis/app/LicenseVault.Server.dll --init
fi
chown -R licentiekluis:licentiekluis /var/lib/licentiekluis
cat > /etc/systemd/system/licentiekluis.service <<'UNIT'
[Unit]
Description=LicentieKluis synchronisatieserver
After=network.target
[Service]
User=licentiekluis
Group=licentiekluis
WorkingDirectory=/opt/licentiekluis/app
ExecStart=/opt/licentiekluis/dotnet/dotnet /opt/licentiekluis/app/LicenseVault.Server.dll
Environment=DOTNET_ROOT=/opt/licentiekluis/dotnet
Environment=ASPNETCORE_URLS=http://127.0.0.1:5080
Environment=LKV_DATA=/var/lib/licentiekluis
Environment=ASPNETCORE_ENVIRONMENT=Production
Restart=on-failure
RestartSec=5
UMask=0077
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/licentiekluis
[Install]
WantedBy=multi-user.target
UNIT
cat > /etc/caddy/Caddyfile <<CADDY
https://$server_name {
    tls internal
    reverse_proxy 127.0.0.1:5080
}
CADDY
chmod 644 /etc/caddy/Caddyfile /etc/systemd/system/licentiekluis.service
chmod 755 /etc/caddy
caddy validate --config /etc/caddy/Caddyfile
systemctl daemon-reload
systemctl enable --now licentiekluis
systemctl restart caddy
systemctl enable caddy
for attempt in {1..20}; do
    if curl -fsS http://127.0.0.1:5080/health >/dev/null; then break; fi
    sleep 1
done
curl -fsS http://127.0.0.1:5080/health
ca=/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt
for attempt in {1..10}; do [[ -f "$ca" ]] && break; sleep 1; done
if [[ -f "$ca" ]]; then
    install -m 644 "$ca" /root/licentiekluis-ca.crt
    echo
    echo 'Kopieer /root/licentiekluis-ca.crt naar Windows en vertrouw dit certificaat (zie handleiding).'
else echo 'CA-certificaat ontbreekt; controleer journalctl -u caddy.' >&2; exit 1; fi
systemctl is-active --quiet licentiekluis caddy
for path in /health / /app.js /app.css /mail.js; do
    curl --fail --silent --show-error --noproxy '*' --connect-timeout 5 --max-time 15 \
        --cacert "$ca" --resolve "$server_name:443:127.0.0.1" "https://$server_name$path" -o /dev/null
done
echo "Server: https://$server_name"
echo 'De .NET runtime is lokaal geinstalleerd. Bij bouwen uit broncode is ook de SDK aanwezig.'

bash "$source_dir/scripts/register-upgrade.sh"
echo "Voortaan bijwerken met: upgrade"

