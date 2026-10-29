#!/bin/sh
# BaseOS generic frontend entry. frontend-session invokes this through /bin/sh
# from the SD root and respawns it after return.
set -u

SD=/mnt/sdcard
RUNTIME="$SD/System/pocketjs/bin/pocket-linux-kms"
PACKAGE="$SD/System/pocketjs/apps/boot.pocket"
LOG=/tmp/pocketjs-frontend.log
FAILURES=/tmp/pocketjs-frontend-failures

log() {
    timestamp=$(date '+%Y-%m-%dT%H:%M:%S' 2>/dev/null || printf unknown-time)
    printf '%s %s\n' "$timestamp" "$*" >> "$LOG" 2>/dev/null || :
}

read_failures() {
    failures=0
    if [ -r "$FAILURES" ]; then
        read -r failures < "$FAILURES" || failures=0
    fi
    case "$failures" in ''|*[!0-9]*) failures=0 ;; esac
}

backoff() {
    read_failures
    failures=$((failures + 1))
    printf '%s\n' "$failures" > "$FAILURES" 2>/dev/null || :
    if [ "$failures" -ge 3 ]; then
        log "failure streak=$failures; sleeping 30 seconds before respawn"
        sleep 30
    else
        sleep 3
    fi
}

log "starting; package=$PACKAGE"
if [ -r "$SD/baseos.conf" ]; then
    log "baseos.conf readable; SSH/ADB are enabled by BaseOS defaults"
else
    log "baseos.conf unavailable; SSH/ADB are enabled by BaseOS defaults"
fi

if [ ! -x "$RUNTIME" ]; then
    log "runtime missing or not executable: $RUNTIME"
    backoff
    exit 1
fi
if [ ! -f "$PACKAGE" ]; then
    log "package missing: $PACKAGE"
    backoff
    exit 1
fi

card=
for candidate in /dev/dri/card0 /dev/dri/card1; do
    if [ -e "$candidate" ]; then
        card=$candidate
        break
    fi
done
if [ -z "$card" ]; then
    log "DRM card not found in /dev/dri/card0..card1"
    backoff
    exit 1
fi
log "DRM card=$card"
if [ -d /storage ] && command -v mountpoint >/dev/null 2>&1 \
    && ! mountpoint -q /storage 2>/dev/null; then
    if mount --bind "$SD" /storage >> "$LOG" 2>&1; then
        log "bound $SD to /storage for the compiled PocketShell bridge"
    else
        log "WARNING: could not bind SD at /storage; launcher state and bridge paths may be unavailable"
    fi
else
    log "/storage bind skipped (mount point missing or already mounted)"
fi



admission_output=$("$RUNTIME" --package "$PACKAGE" --check 2>&1)
admission_status=$?
log "admission exit=$admission_status output=$admission_output"
if [ "$admission_status" -ne 0 ]; then
    log "package admission failed; runtime will not be started"
    backoff
    exit 1
fi

(
    exec "$RUNTIME" --package "$PACKAGE" --card "$card"
) >> "$LOG" 2>&1 &
runtime_pid=$!
wait "$runtime_pid"
runtime_status=$?
log "PocketJS exit code=$runtime_status"
read_failures
failures=$((failures + 1))
printf '%s\n' "$failures" > "$FAILURES" 2>/dev/null || :
if [ "$failures" -ge 3 ]; then
    log "failure streak=$failures; sleeping 30 seconds before respawn"
    sleep 30
else
    sleep 3
fi
exit "$runtime_status"
