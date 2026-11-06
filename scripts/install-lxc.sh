#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
[[ $EUID -eq 0 ]] || { echo 'Voer uit als root binnen de LXC.'; exit 1; }
action=${1:-install}
[[ "$action" == install || "$action" == upgrade ]] || { echo 'Gebruik: install-lxc.sh [install|upgrade]'; exit 1; }
command -v curl >/dev/null || { apt-get update; apt-get install -y curl ca-certificates; }
command -v flock >/dev/null || { echo 'Installeer util-linux voor flock.'; exit 1; }
exec 9>/run/lock/licentiekluis-upgrade.lock
flock -n 9 || { echo 'Er draait al een installatie of upgrade.'; exit 1; }
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
token=${GITHUB_TOKEN:-}
if [[ -z "$token" && -f /etc/licentiekluis/github-token ]]; then
    [[ $(stat -c '%u:%a' /etc/licentiekluis/github-token) == 0:600 ]] || { echo 'Tokenbestand moet root:root en chmod 600 hebben.'; exit 1; }
    token=$(cat /etc/licentiekluis/github-token)
fi
if [[ -z "$token" && -t 0 ]]; then
    read -rsp 'GitHub-token (priverepository, Enter indien openbaar): ' token
    echo
fi
[[ "$token" != *$'\n'* && "$token" != *$'\r'* && "$token" != *'"'* && "$token" != *'\'* ]] || { echo 'Ongeldig token.'; exit 1; }
printf 'header = "Accept: application/vnd.github+json"\n' > "$work/curl.conf"
if [[ -n "$token" ]]; then printf 'header = "Authorization: Bearer %s"\n' "$token" >> "$work/curl.conf"; fi
unset token GITHUB_TOKEN
# GitHub resolves main once; the archive contains that exact commit.
curl --config "$work/curl.conf" --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 \
    https://api.github.com/repos/MadNeo/licentiekluis/tarball/main -o "$work/source.tar.gz"
mkdir "$work/source"
tar -xzf "$work/source.tar.gz" -C "$work/source" --strip-components=1
[[ -f "$work/source/Server/LicenseVault.Server.csproj" ]] || { echo 'Ongeldig GitHub-pakket.'; exit 1; }
tar -tzf "$work/source.tar.gz" | head -n 1 > "$work/archive-name" || true
cat "$work/archive-name" > "$work/source/GITHUB-REVISION"
if [[ "$action" == install ]]; then
    bash "$work/source/scripts/install-local.sh"
else
    [[ -f /etc/systemd/system/licentiekluis.service ]] || { echo "LicentieKluis-service ontbreekt; maak eerst de serverinstallatie af." >&2; exit 1; }
    # Repair missing upgrade commands before attempting the build.
    bash "$work/source/scripts/register-upgrade.sh"
    bash "$work/source/scripts/update-server.sh"
fi

