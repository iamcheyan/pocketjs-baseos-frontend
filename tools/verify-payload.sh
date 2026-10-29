#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fail() { printf 'verify-payload: FAIL: %s\n' "$*" >&2; exit 1; }

[ -f "$ROOT/inputs/pocket-linux-kms" ] || fail 'runtime input missing'
[ -x "$ROOT/inputs/pocket-linux-kms" ] || fail 'runtime input not executable'
[ -f "$ROOT/inputs/boot.pocket" ] || fail 'boot.pocket input missing'
[ -f "$ROOT/System/launch_frontend.sh" ] || fail 'frontend script missing'

if grep -Eiq 'systemctl|python|/storage/' "$ROOT/System/launch_frontend.sh"; then
    fail 'launch_frontend.sh contains forbidden systemctl/python//storage/ dependency'
fi

find "$ROOT/System" "$ROOT/tools" -type f -name '*.sh' -print | while IFS= read -r script; do
    sh -n "$script" || fail "shell syntax failed: $script"
done

if command -v busybox >/dev/null 2>&1; then
    busybox sh -n "$ROOT/System/launch_frontend.sh" || fail 'busybox ash syntax failed'
fi

printf 'verify-payload: PASS\n'
printf 'runtime input: inputs/pocket-linux-kms (%s bytes, executable)\n' \
    "$(wc -c < "$ROOT/inputs/pocket-linux-kms" | tr -d ' ')"
printf 'package input: inputs/boot.pocket (%s bytes)\n' \
    "$(wc -c < "$ROOT/inputs/boot.pocket" | tr -d ' ')"
printf 'shell scripts: sh -n passed\n'
if command -v busybox >/dev/null 2>&1; then printf 'shell scripts: busybox ash -n passed\n'; fi
printf 'forbidden launcher dependencies: none\n'
