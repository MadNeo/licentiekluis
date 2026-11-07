#!/bin/sh
set -eu
umask 077

data_dir=${LKV_DATA:-/data}
export LKV_DATA="$data_dir"
if [ -n "${RAILWAY_ENVIRONMENT_ID:-}" ] && [ "${RAILWAY_VOLUME_MOUNT_PATH:-}" != "$data_dir" ]; then
    echo "Koppel eerst een Railway-volume op $data_dir." >&2
    exit 1
fi

if [ ! -f "$data_dir/secrets.json" ]; then
    if [ -f "$data_dir/server.lkv" ]; then
        echo "De serverkluis is aanwezig. Zet ook de bijbehorende secrets.json terug." >&2
        exit 1
    fi
    : "${LKV_ADMIN_USER:?Stel LKV_ADMIN_USER in voor de eerste start.}"
    : "${LKV_ADMIN_PASSWORD:?Stel LKV_ADMIN_PASSWORD in voor de eerste start.}"
    printf '%s\n' "$LKV_ADMIN_USER" "$LKV_ADMIN_PASSWORD" "$LKV_ADMIN_PASSWORD" |
        dotnet LicenseVault.Server.dll --init-stdin
fi
unset LKV_ADMIN_USER LKV_ADMIN_PASSWORD
export ASPNETCORE_URLS="http://0.0.0.0:${PORT:-8080}"
exec dotnet LicenseVault.Server.dll
