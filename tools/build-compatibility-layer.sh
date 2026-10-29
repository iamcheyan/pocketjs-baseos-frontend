#!/bin/sh
# Stage only compatibility helpers that can be inspected without editing the
# BaseOS source tree or installing anything on a device.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT="$ROOT/dist/compat"
mkdir -p "$OUT/usr/bin" "$OUT/usr/sbin"

cat > "$OUT/usr/bin/pocketshell-games" <<'SH'
#!/bin/sh
# Emit the games JSON catalogue shape consumed by pocketshell-launcher.rs.
set -eu
ROOT=${POCKETSHELL_GAMES_ROOT:-/storage/games}
INDEX=${POCKETSHELL_GAMES_INDEX:-/storage/pocketshell/library/library.json}
[ "${1-}" = scan ] || { echo 'usage: pocketshell-games scan' >&2; exit 2; }
mkdir -p "${INDEX%/*}"
tmp="$INDEX.tmp.$$"
trap 'rm -f "$tmp"' EXIT HUP INT TERM
export LC_ALL=C
printf '{"sections":{"games":[' > "$tmp"
first_group=1
total=0
emit_group() {
    group_name=$1
    group_dir=$2
    items=
    count=0
    first_item=1
    for entry in "$group_dir"/*; do
        [ -f "$entry" ] || continue
        base=${entry##*/}
        [ "$base" = '*' ] && continue
        case "$base" in .* ) continue ;; esac
        extension=${base##*.}
        case "$extension" in
            gba|GBA|gbc|GBC|gb|GB|nes|NES|sfc|SFC|smc|SMC|md|MD|gen|GEN|sms|SMS|gg|GG|pce|PCE|ngp|NGP|ngc|NGC|ws|WS|wsc|WSC|lnx|LNX|a26|A26|a78|A78|cue|CUE|chd|CHD|pbp|PBP|iso|ISO|zip|ZIP|7z|cso|rvz|gcm|n64|z64|v64|nds|3ds|cia|xci|nsp|vb|min|col|int|pocket|POCKET) ;;
            *) continue ;;
        esac
        title=${base%.*}
        escaped_title=$(printf '%s' "$title" | sed 's/\\/\\\\/g; s/"/\\"/g')
        escaped_path=$(printf '%s' "$entry" | sed 's/\\/\\\\/g; s/"/\\"/g')
        item=$(printf '{"title":"%s","path":"%s"}' "$escaped_title" "$escaped_path")
        if [ "$first_item" -eq 1 ]; then items=$item; first_item=0; else items="$items,$item"; fi
        count=$((count + 1))
    done
    [ "$count" -gt 0 ] || return 0
    escaped_group=$(printf '%s' "$group_name" | sed 's/\\/\\\\/g; s/"/\\"/g')
    [ "$first_group" -eq 1 ] || printf ',' >> "$tmp"
    printf '{"name":"%s","count":%s,"items":[%s]}' "$escaped_group" "$count" "$items" >> "$tmp"
    first_group=0
    total=$((total + count))
}
for dir in "$ROOT"/*; do
    [ -d "$dir" ] || continue
    system=${dir##*/}
    [ "$system" = '*' ] && continue
    case "$system" in .*|images|media|gamelist|downloaded_images|videos|manuals|cheats|system|saves) continue ;; esac
    emit_group "$system" "$dir"
done
emit_group misc "$ROOT"
printf ']},"total":%s}\n' "$total" >> "$tmp"
cat "$tmp"
mv "$tmp" "$INDEX"
SH

cat > "$OUT/usr/sbin/pocketjs-systemctl-status" <<'SH'
#!/bin/sh
# Status mapping helper for a reviewed BaseOS systemctl integration.
set -eu
case "${1-}" in
    is-active)
        unit=${2-}
        case "$unit" in
            ssh.service|dropbear.service) pidof dropbear >/dev/null 2>&1; exit $? ;;
            adbd.service|adbd) pidof adbd >/dev/null 2>&1; exit $? ;;
            avahi-daemon.service|avahi-daemon) pidof avahi-daemon >/dev/null 2>&1; exit $? ;;
            *) exit 3 ;;
        esac
        ;;
    *) exit 2 ;;
esac
SH

for shim in "$OUT/usr/bin/pocketshell-games" "$OUT/usr/sbin/pocketjs-systemctl-status"; do
    chmod 0755 "$shim"
    sh -n "$shim"
done
printf 'safe compatibility helpers staged under %s (not installed)\n' "$OUT"
