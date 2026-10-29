#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
IN="$ROOT/inputs"
OUT="$ROOT/dist"
STAGE="$OUT/sdcard-payload"

[ -f "$IN/pocket-linux-kms" ] || { printf 'missing %s\n' "$IN/pocket-linux-kms" >&2; exit 1; }
[ -f "$IN/boot.pocket" ] || { printf 'missing %s\n' "$IN/boot.pocket" >&2; exit 1; }
[ -x "$ROOT/tools/verify-payload.sh" ] || chmod +x "$ROOT/tools/verify-payload.sh"
"$ROOT/tools/verify-payload.sh"

rm -rf "$STAGE"
mkdir -p "$STAGE/System/pocketjs/bin" "$STAGE/System/pocketjs/apps"
cp "$ROOT/System/launch_frontend.sh" "$STAGE/System/launch_frontend.sh"
cp "$IN/pocket-linux-kms" "$STAGE/System/pocketjs/bin/pocket-linux-kms"
cp "$IN/boot.pocket" "$STAGE/System/pocketjs/apps/boot.pocket"
if [ -d "$ROOT/System/compat" ]; then
    cp -a "$ROOT/System/compat" "$STAGE/System/compat"
fi
chmod 0755 "$STAGE/System/launch_frontend.sh" "$STAGE/System/pocketjs/bin/pocket-linux-kms"
{
    printf 'Runtime source: inputs/pocket-linux-kms\n'
    printf 'Runtime SHA-256: %s\n' "$(sha256sum "$IN/pocket-linux-kms" | cut -d ' ' -f 1)"
    printf 'Launcher package source: inputs/boot.pocket\n'
    printf 'Launcher package size: %s bytes\n' "$(wc -c < "$IN/boot.pocket" | tr -d ' ')"
    printf 'Launcher package SHA-256: %s\n' "$(sha256sum "$IN/boot.pocket" | cut -d ' ' -f 1)"
    printf 'Package profile: rocknix-h700-kms; verify admission on target\n'
} > "$STAGE/System/pocketjs/version"
cp "$ROOT/README.txt" "$STAGE/README.txt"

mkdir -p "$OUT"
TMP="$OUT/sdcard-payload.tar.gz.tmp"
rm -f "$TMP"
tar -C "$STAGE" -czf "$TMP" System README.txt
mv "$TMP" "$OUT/sdcard-payload.tar.gz"
bytes=$(wc -c < "$OUT/sdcard-payload.tar.gz" | tr -d ' ')
sha=$(sha256sum "$OUT/sdcard-payload.tar.gz" | cut -d ' ' -f 1)
printf 'payload: %s\nsize: %s bytes\nsha256: %s\n' "$OUT/sdcard-payload.tar.gz" "$bytes" "$sha"
