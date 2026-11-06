#!/usr/bin/env bash
# Register the updater without changing the application or its data.
set -Eeuo pipefail
[[ $EUID -eq 0 ]] || { echo 'Voer dit script als root in de LicentieKluis-container uit.' >&2; exit 1; }
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
[[ -s "$source_dir/scripts/install-lxc.sh" ]] || { echo 'GitHub-installer ontbreekt; registratie afgebroken.' >&2; exit 1; }
bash -n "$source_dir/scripts/install-lxc.sh"
# Never replace an unrelated command, including a dangling symlink.
for target in /usr/local/bin/upgrade /usr/bin/upgrade; do
    if [[ -e "$target" || -L "$target" ]]; then
        if [[ -L "$target" && $(readlink "$target") == /usr/local/bin/upgrade ]]; then
            continue
        fi
        if [[ ! -f "$target" ]] || ! grep -q '^# LicentieKluis upgrade$' "$target"; then
            echo "$target bestaat al en is niet van LicentieKluis; niets overschreven." >&2
            exit 1
        fi
    fi
done
install -d -m 755 /opt/licentiekluis /usr/local/bin
stage=$(mktemp -d /opt/licentiekluis/register-upgrade.XXXXXX)
wrapper=$(mktemp /usr/local/bin/.licentiekluis-upgrade.XXXXXX)
trap 'rm -rf -- "$stage"; rm -f -- "$wrapper"' EXIT
install -m 700 "$source_dir/scripts/install-lxc.sh" "$stage/github-installer.sh"
cat > "$wrapper" <<'WRAPPER'
#!/usr/bin/env bash
# LicentieKluis upgrade
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
    echo 'LicentieKluis upgrade: voer upgrade als root in de LXC uit.'
    echo 'Haalt de actuele GitHub-versie op en behoudt database, accounts en instellingen.'
    exit 0
fi
exec bash /opt/licentiekluis/github-installer.sh upgrade "$@"
WRAPPER
chmod 755 "$wrapper"
bash -n "$wrapper"
mv -f -- "$stage/github-installer.sh" /opt/licentiekluis/github-installer.sh
mv -f -- "$wrapper" /usr/local/bin/upgrade
# Some container consoles omit /usr/local/bin from PATH.
ln -sfn /usr/local/bin/upgrade /usr/bin/upgrade
/usr/local/bin/upgrade --help
