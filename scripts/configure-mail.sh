#!/usr/bin/env bash
set -Eeuo pipefail
[[ $EUID -eq 0 ]] || { echo 'Voer dit uit als root BINNEN de LXC.'; exit 1; }
[[ -f /var/lib/licentiekluis/secrets.json ]] || { echo 'Installeer eerst de server.'; exit 1; }
umask 077
systemctl stop licentiekluis
trap 'chown -R licentiekluis:licentiekluis /var/lib/licentiekluis; systemctl start licentiekluis' EXIT
LKV_DATA=/var/lib/licentiekluis /opt/licentiekluis/dotnet/dotnet \
  /opt/licentiekluis/app/LicenseVault.Server.dll --configure-mail
