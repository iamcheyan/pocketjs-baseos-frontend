#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fail() { printf 'verify-payload: FAIL: %s\n' "$*" >&2; exit 1; }

[ -x "$ROOT/System/pocketjs/bin/pocket-linux-kms" ] || fail 'runtime missing or not executable'
[ -f "$ROOT/System/pocketjs/apps/boot.pocket" ] || fail 'boot.pocket missing'
[ -f "$ROOT/System/pocketjs/version" ] || fail 'runtime version record missing'
[ -f "$ROOT/System/launch_frontend.sh" ] || fail 'frontend script missing'

if grep -Eiq 'systemctl|python|/storage/|pocket-session-run' "$ROOT/System/launch_frontend.sh"; then
    fail 'launch_frontend.sh contains a forbidden BaseOS or ROCKNIX dependency'
fi

find "$ROOT/System" "$ROOT/tools" -type f -name '*.sh' -print | while IFS= read -r script; do
    sh -n "$script" || fail "shell syntax failed: $script"
done

if command -v busybox >/dev/null 2>&1; then
    busybox sh -n "$ROOT/System/launch_frontend.sh" || fail 'busybox ash syntax failed'
fi

printf 'verify-payload: PASS\n'
printf 'runtime: System/pocketjs/bin/pocket-linux-kms (%s bytes, executable)\n' \
    "$(wc -c < "$ROOT/System/pocketjs/bin/pocket-linux-kms" | tr -d ' ')"
printf 'package: System/pocketjs/apps/boot.pocket (%s bytes)\n' \
    "$(wc -c < "$ROOT/System/pocketjs/apps/boot.pocket" | tr -d ' ')"
printf 'shell scripts: sh -n passed\n'
if command -v busybox >/dev/null 2>&1; then printf 'shell scripts: busybox ash -n passed\n'; fi
printf 'forbidden launcher dependencies: none\n'
