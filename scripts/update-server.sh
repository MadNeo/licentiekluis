#!/usr/bin/env bash
# Internal deployment step; use upgrade to fetch the latest GitHub source.
set -Eeuo pipefail
umask 077
[[ $EUID -eq 0 && -f /etc/systemd/system/licentiekluis.service ]] || { echo 'Voer upgrade uit als root op de bestaande server.'; exit 1; }
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
[[ -f "$source_dir/GITHUB-REVISION" ]] || { exec bash "$source_dir/scripts/install-lxc.sh" upgrade; }
dotnet=/opt/licentiekluis/dotnet/dotnet
export DOTNET_ROOT=/opt/licentiekluis/dotnet
stage=$(mktemp -d /opt/licentiekluis/stage.XXXXXX)
backup_dir=$(mktemp -d /opt/licentiekluis/backup-$(date +%Y%m%d-%H%M%S).XXXXXX)
stopped=0
swapped=0
committed=0
cleanup() {
    result=$?
    trap - EXIT
    set +e
    if (( stopped && ! committed )); then
        systemctl stop licentiekluis
        if (( swapped )); then
            if [[ -d /opt/licentiekluis/app ]]; then mv /opt/licentiekluis/app "$backup_dir/failed-app"; fi
            if ! cp -a "$backup_dir/app" /opt/licentiekluis/app; then echo 'Programmaherstel mislukt; server blijft gestopt.' >&2; exit 1; fi
            mv /var/lib/licentiekluis "$backup_dir/failed-data"
            if ! tar -C /var/lib -xzf "$backup_dir/data.tar.gz"; then
                echo "HERSTEL MISLUKT. Server blijft gestopt. Back-up: $backup_dir" >&2
                exit 1
            fi
        fi
        systemctl start licentiekluis || echo 'Controleer journalctl -u licentiekluis.' >&2
        echo "Upgrade afgebroken; vorige installatie hersteld. Back-up: $backup_dir" >&2
    fi
    rm -rf -- "$stage"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
curl --fail --show-error --location --proto '=https' --tlsv1.2 https://dot.net/v1/dotnet-install.sh -o "$stage/dotnet-install.sh"
bash "$stage/dotnet-install.sh" --channel 10.0 --install-dir "$DOTNET_ROOT" --no-path
chmod -R a+rX "$DOTNET_ROOT"
"$dotnet" run --project "$source_dir/Tests/LicenseVault.Tests.csproj" -c Release
"$dotnet" publish "$source_dir/Server/LicenseVault.Server.csproj" -c Release --self-contained false -p:UseAppHost=false -o "$stage/app"
cp "$source_dir/GITHUB-REVISION" "$stage/app/GITHUB-REVISION"
chmod -R a+rX "$stage/app"
systemctl stop licentiekluis
stopped=1
tar -C /var/lib -czf "$backup_dir/data.tar.gz" licentiekluis
cp -a /etc/systemd/system/licentiekluis.service "$backup_dir/"
cp -a /etc/caddy/Caddyfile "$backup_dir/"
cp -a /opt/licentiekluis/app "$backup_dir/app"
mv /opt/licentiekluis/app "$stage/old-app"
swapped=1
mv "$stage/app" /opt/licentiekluis/app
systemctl start licentiekluis
healthy=0
for attempt in {1..30}; do
    if systemctl is-active --quiet licentiekluis && curl -fsS http://127.0.0.1:5080/health >/dev/null; then healthy=1; break; fi
    sleep 1
done
(( healthy )) || { echo 'Nieuwe server niet gezond; herstel wordt uitgevoerd.'; exit 1; }
bash "$source_dir/scripts/register-upgrade.sh"
committed=1
echo "Upgrade gereed. Database, accounts, sleutels en SMTP-instellingen behouden. Back-up: $backup_dir"
