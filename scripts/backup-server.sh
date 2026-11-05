#!/usr/bin/env bash
set -Eeuo pipefail
umask 077
[[ $EUID -eq 0 ]] || { echo 'Root vereist.'; exit 1; }
destination=${1:?Geef een doelmap voor de back-up op}
mkdir -p -- "$destination"
destination=$(realpath -- "$destination")
case "$destination" in /var/lib/licentiekluis|/var/lib/licentiekluis/*) echo 'Kies een doelmap buiten de bronmap.'; exit 1;; esac
systemctl stop licentiekluis
trap 'systemctl start licentiekluis' EXIT
tar -C /var/lib -czf "$destination/licentiekluis-$(date +%Y%m%d-%H%M%S).tar.gz" licentiekluis
echo 'Back-up bevat ook de serversleutel. Bewaar hem afgeschermd en versleutel het back-upmedium.'
