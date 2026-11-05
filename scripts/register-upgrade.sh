#!/usr/bin/env bash
set -Eeuo pipefail
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ -e /usr/local/bin/upgrade ]] && ! grep -q 'LicentieKluis upgrade' /usr/local/bin/upgrade; then
    echo '/usr/local/bin/upgrade bestaat al en is niet van LicentieKluis.' >&2
    exit 1
fi
install -m 700 "$source_dir/scripts/install-lxc.sh" /opt/licentiekluis/github-installer.sh
cat > /usr/local/bin/upgrade <<'WRAPPER'
#!/usr/bin/env bash
# LicentieKluis upgrade
exec bash /opt/licentiekluis/github-installer.sh upgrade "$@"
WRAPPER
chmod 755 /usr/local/bin/upgrade
