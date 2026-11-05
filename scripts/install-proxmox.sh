#!/usr/bin/env bash
# Run on the Proxmox VE host. Never overwrites or destroys an existing guest.
set -Eeuo pipefail
umask 077
REPOSITORY=MadNeo/licentiekluis
# The application and future upgrades always use main, resolved once per download.
SOURCE_REF=main
work=''; created=0; complete=0; ctid=''
fail() { printf 'FOUT: %s\n' "$*" >&2; exit 1; }
cleanup() {
    rc=$?
    trap - EXIT
    if (( created )); then
        timeout --kill-after=1 5 pct exec "$ctid" -- rm -f /root/licentiekluis-init.txt 2>/dev/null || true
        if (( ! complete )); then
            printf '\nInstallatie niet voltooid. Container %s is behouden voor onderzoek.\n' "$ctid" >&2
            printf 'Open de console met: pct enter %s\nEr is geen bestaande container verwijderd.\n' "$ctid" >&2
        fi
    fi
    [[ -z "$work" ]] || rm -rf -- "$work"
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
ask() {
    local var=$1 label=$2 default=$3 answer
    read -rp "$label [$default]: " answer
    printf -v "$var" '%s' "${answer:-$default}"
}
valid_id() { [[ $1 =~ ^[1-9][0-9]{2,8}$ ]]; }
valid_name() { [[ $1 =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$ ]]; }
active_storage() { pvesm status --content "$1" | awk 'NR>1 && $3=="active" {print $1}'; }
choose_storage() {
    local var=$1 content=$2 label=$3 list choice first
    list=$(active_storage "$content")
    [[ -n "$list" ]] || fail "Geen actieve opslag voor $content. Configureer opslag in Proxmox."
    printf '%s:\n%s\n' "$label" "$list"
    first=$(printf '%s\n' "$list" | head -n 1)
    ask choice 'Opslag kiezen' "$first"
    grep -Fxq -- "$choice" <<< "$list" || fail 'Kies een opslag uit de lijst.'
    printf -v "$var" '%s' "$choice"
}
main() {
    [[ $EUID -eq 0 ]] || fail 'Open de Shell van de Proxmox-node als root.'
    [[ -d /etc/pve ]] && command -v pveversion >/dev/null || fail 'Dit script hoort op een bestaande Proxmox VE-host, niet in een LXC of op gewone Ubuntu.'
    [[ $(uname -m) == x86_64 ]] || fail 'Alleen x86_64/amd64 wordt ondersteund.'
    [[ -t 0 ]] || fail 'Start interactief: bash install-proxmox.sh (niet via een pipe).'
    local packages=() tool pkg
    for tool in curl python3 tar flock pct pveam; do
        if ! command -v "$tool" >/dev/null; then
            case "$tool" in flock) pkg=util-linux;; pct|pveam) pkg=pve-container;; *) pkg=$tool;; esac
            packages+=("$pkg")
        fi
    done
    if ! dpkg-query -W -f='${Status}' ca-certificates 2>/dev/null | grep -q 'install ok installed'; then packages+=(ca-certificates); fi
    if (( ${#packages[@]} )); then apt-get update; apt-get install -y "${packages[@]}"; fi
    for tool in pct pveam pvesm pvesh; do command -v "$tool" >/dev/null || fail "$tool ontbreekt; herstel eerst de Proxmox-installatie."; done
    exec 9>/run/lock/licentiekluis-proxmox.lock
    flock -n 9 || fail 'Er draait al een LicentieKluis-containerinstallatie op deze node.'
    local suggested hostname root_storage template_storage bridge cidr gateway cores memory disk vlan net server_name admin password repeat token template archive ip ca
    suggested=$(pvesh get /cluster/nextid)
    ask ctid 'Container-ID' "$suggested"
    valid_id "$ctid" || fail 'Ongeldig container-ID.'
    pvesh get /cluster/nextid --vmid "$ctid" >/dev/null || fail 'Dit ID is al in gebruik in het cluster.'
    ask hostname 'Containernaam' licentiekluis
    valid_name "$hostname" || fail 'Ongeldige containernaam.'
    choose_storage root_storage rootdir 'Beschikbare opslag voor de containerdisk'
    choose_storage template_storage vztmpl 'Beschikbare opslag voor Ubuntu-templates'
    printf 'Netwerkbridges:\n'
    # Inspect only the bridge marker; never recursively follow sysfs symlinks.
    local bridge_path
    for bridge_path in /sys/class/net/*; do
        if [[ -d "$bridge_path/bridge" ]]; then
            printf '%s\n' "${bridge_path##*/}"
        fi
    done
    ask bridge 'Netwerkbridge' vmbr0
    [[ "$bridge" =~ ^[a-zA-Z0-9_.-]+$ && -d /sys/class/net/"$bridge"/bridge ]] || fail 'Deze netwerkbridge bestaat niet.'
    ask cidr 'IPv4/CIDR of dhcp (vast IP aanbevolen)' dhcp
    gateway=''
    if [[ "$cidr" != dhcp ]]; then
        python3 -c 'import ipaddress,sys; ipaddress.IPv4Interface(sys.argv[1]); assert "/" in sys.argv[1]' "$cidr" || fail 'Gebruik bijvoorbeeld 192.168.178.50/24.'
        ask gateway 'IPv4-gateway' 192.168.178.1
        python3 -c 'import ipaddress,sys; ipaddress.IPv4Address(sys.argv[1])' "$gateway" || fail 'Ongeldige gateway.'
    fi
    ask vlan 'VLAN-ID (0 = geen VLAN-tag)' 0
    [[ "$vlan" =~ ^[0-9]{1,4}$ ]] && (( 10#$vlan <= 4094 )) || fail 'VLAN moet 0 t/m 4094 zijn.'
    ask cores 'CPU-kernen' 2; ask memory 'RAM in MB' 2048; ask disk 'Disk in GB' 8
    [[ "$cores" =~ ^[1-9][0-9]*$ && "$memory" =~ ^[1-9][0-9]*$ && "$disk" =~ ^[1-9][0-9]*$ ]] || fail 'CPU, RAM en disk moeten positieve gehele getallen zijn.'
    (( cores <= 128 && memory >= 1024 && disk >= 8 )) || fail 'Gebruik maximaal 128 CPU-kernen, minimaal 1024 MB RAM en 8 GB disk.'
    read -rp 'Server-DNS-naam (Enter = het IPv4-adres van de container): ' server_name
    [[ -z "$server_name" || "$server_name" =~ ^[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?$ ]] || fail 'Ongeldige servernaam; geen https:// of pad invoeren.'
    read -rp 'Nieuw LicentieKluis-serveraccount: ' admin
    [[ -n "$admin" && ${#admin} -le 200 ]] || fail 'Vul een serveraccount in (maximaal 200 tekens).'
    read -rsp 'Serverwachtwoord (minimaal 14 tekens): ' password; echo
    read -rsp 'Herhaal serverwachtwoord: ' repeat; echo
    [[ ${#password} -ge 14 && "$password" == "$repeat" ]] || fail 'Wachtwoorden verschillen of zijn te kort.'
    token=${GITHUB_TOKEN:-}
    unset GITHUB_TOKEN
    if [[ -z "$token" ]]; then
        read -rsp 'GitHub-token met Contents: Read voor MadNeo/licentiekluis (Enter bij openbare repository): ' token; echo
    fi
    [[ "$token" != *$'\r'* && "$token" != *'"'* && "$token" != *'\'* ]] || fail 'Ongeldig token.'
    printf '\nNieuwe LXC %s (%s): %s CPU, %s MB RAM, %s GB op %s\nNetwerk: %s, %s, VLAN %s\n' "$ctid" "$hostname" "$cores" "$memory" "$disk" "$root_storage" "$bridge" "$cidr" "$vlan"
    printf 'Ubuntu 24.04; onbevoorrecht; automatisch starten bij boot.\nGitHub-token wordt alleen in de LXC als root-only bestand opgeslagen voor upgrade.\n'
    local answer
    read -rp 'Deze nieuwe container aanmaken? [ja/nee]: ' answer
    [[ "$answer" == ja ]] || fail 'Geannuleerd; geen container aangemaakt.'
    work=$(mktemp -d)
    printf 'header = "Accept: application/vnd.github+json"\n' > "$work/curl.conf"
    if [[ -n "$token" ]]; then printf 'header = "Authorization: Bearer %s"\n' "$token" >> "$work/curl.conf"; fi
    # Download before allocating a container: fail early on missing repository access.
    curl --config "$work/curl.conf" --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 \
        "https://api.github.com/repos/$REPOSITORY/tarball/$SOURCE_REF" -o "$work/source.tar.gz"
    mkdir "$work/source"
    tar -xzf "$work/source.tar.gz" -C "$work/source" --strip-components=1
    [[ -f "$work/source/scripts/install-local.sh" && -f "$work/source/Server/LicenseVault.Server.csproj" ]] || fail 'GitHub-pakket is onvolledig.'
    tar -tzf "$work/source.tar.gz" > "$work/archive-list"
    head -n 1 "$work/archive-list" > "$work/source/GITHUB-REVISION"
    grep -q 'LKV_INIT_FILE' "$work/source/scripts/install-local.sh" || fail 'GitHub main bevat nog niet de nieuwe Proxmox-installer.'
    tar -czf "$work/deploy.tar.gz" -C "$work/source" .
    pveam update
    archive=$(pveam available --section system | awk '$2 ~ /^ubuntu-24\.04-standard_.*_amd64\.tar\.(zst|xz|gz)$/ {print $2}' | sort -V | tail -n 1)
    [[ -n "$archive" ]] || fail 'Geen Ubuntu 24.04 amd64-template gevonden in Proxmox.'
    pveam download "$template_storage" "$archive"
    template="$template_storage:vztmpl/$archive"
    net="name=eth0,bridge=$bridge,ip=$cidr,type=veth"
    [[ -z "$gateway" ]] || net+=",gw=$gateway"
    (( 10#$vlan == 0 )) || net+=",tag=$((10#$vlan))"
    # pct itself rejects an occupied ID; never stop/destroy an existing guest.
    # Proxmox generates system/network files using the inherited umask.
    # Keep secrets at 077, but create/start the guest with normal OS permissions.
    (umask 022; pct create "$ctid" "$template" --hostname "$hostname" --ostype ubuntu --unprivileged 1 \
        --cores "$cores" --memory "$memory" --swap 512 --rootfs "$root_storage:$disk" \
        --net0 "$net" --onboot 1 --description 'LicentieKluis: versleutelde licentieserver; update binnen LXC met upgrade')
    created=1
    (umask 022; pct start "$ctid")
    printf 'Wachten op IPv4-adres van container %s (maximaal circa 3 minuten)...\n' "$ctid"
    ip=''
    for attempt in {1..20}; do
        printf 'Netwerkcontrole %s/20...\n' "$attempt"
        ip=$(timeout --kill-after=1 5 pct exec "$ctid" -- ip -4 -o addr show dev eth0 scope global 2>/dev/null | awk '{split($4,a,"/");print a[1];exit}') || true
        [[ -z "$ip" ]] || break
        sleep 2
    done
    [[ -n "$ip" ]] || fail 'Geen IPv4-adres ontvangen. Controleer bridge, VLAN en DHCP; container is behouden.'
    [[ -n "$server_name" ]] || server_name=$ip
    printf '%s\n%s\n%s\n' "$admin" "$password" "$repeat" > "$work/init.txt"
    unset password repeat
    pct push "$ctid" "$work/init.txt" /root/licentiekluis-init.txt --perms 0600
    pct push "$ctid" "$work/deploy.tar.gz" /root/licentiekluis-source.tar.gz --perms 0600
    pct exec "$ctid" -- mkdir -p /root/licentiekluis-source
    pct exec "$ctid" -- tar -xzf /root/licentiekluis-source.tar.gz -C /root/licentiekluis-source
    if [[ -n "$token" ]]; then
        printf '%s' "$token" > "$work/token"
        pct exec "$ctid" -- install -d -m 700 /etc/licentiekluis
        pct push "$ctid" "$work/token" /etc/licentiekluis/github-token --perms 0600
    fi
    unset token
    pct exec "$ctid" -- env LKV_SERVER_NAME="$server_name" LKV_INIT_FILE=/root/licentiekluis-init.txt \
        bash /root/licentiekluis-source/scripts/install-local.sh
    pct exec "$ctid" -- rm -f /root/licentiekluis-init.txt /root/licentiekluis-source.tar.gz
    ca="/root/licentiekluis-$ctid-ca.crt"
    pct pull "$ctid" /root/licentiekluis-ca.crt "$ca"
    complete=1
    printf '\nGEREED: https://%s\nContainer: %s | IPv4: %s\n' "$server_name" "$ctid" "$ip"
    printf 'Windows: vertrouw eerst het CA-certificaat %s (zie handleiding).\n' "$ca"
    printf 'Console: pct enter %s\nBijwerken binnen die console: upgrade\n' "$ctid"
    [[ "$cidr" != dhcp ]] || printf 'Reserveer dit IP-adres in DHCP om het serveradres stabiel te houden.\n'
}
# Sourceable validation helpers for isolated tests; normal execution runs the wizard.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then main "$@"; fi

