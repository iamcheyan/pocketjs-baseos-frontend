#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PAYLOAD="$ROOT/dist/sdcard-payload.tar.gz"
usage() { printf 'Usage: %s MOUNTED_SD_ROOT\n' "$0" >&2; exit 2; }
[ "$#" -eq 1 ] || usage
TARGET=$1
[ -d "$TARGET" ] || { printf 'deploy-to-card: not a directory: %s\n' "$TARGET" >&2; exit 1; }
[ -w "$TARGET" ] || { printf 'deploy-to-card: not writable: %s\n' "$TARGET" >&2; exit 1; }
[ -f "$PAYLOAD" ] || { printf 'deploy-to-card: missing payload; run tools/make-payload.sh first\n' >&2; exit 1; }

stamp=$(date -u '+%Y%m%dT%H%M%SZ')
BACKUP="$TARGET/.pocketjs-backup-$stamp"
mkdir -p "$BACKUP"
for path in System README.txt; do
    if [ -e "$TARGET/$path" ]; then
        mv "$TARGET/$path" "$BACKUP/$path"
    fi
done

if ! tar -C "$TARGET" -xzf "$PAYLOAD"; then
    printf 'deploy-to-card: extraction failed; restoring backup %s\n' "$BACKUP" >&2
    rm -rf "$TARGET/System" "$TARGET/README.txt"
    for path in System README.txt; do
        [ ! -e "$BACKUP/$path" ] || mv "$BACKUP/$path" "$TARGET/$path"
    done
    exit 1
fi
printf 'deployed payload to %s\nbackup: %s\n' "$TARGET" "$BACKUP"
