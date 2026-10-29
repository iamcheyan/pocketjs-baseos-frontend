# BaseOS PocketJS device checklist

Target: Anbernic RG34XX, 720×480 physical display, 360×240 logical viewport at
2× presentation. This checklist is for Hermes after transferring the payload;
no device-side step has been run from the build workspace.

## Before writing the card

1. Confirm the destination is the intended BaseOS SD card root, with `System/`
   at that root.
2. Preserve the `.pocketjs-backup-*` directory created by `tools/deploy-to-card.sh`
   until the new frontend has booted successfully.
3. Confirm `System/pocketjs/version` records the approved package hash.

## First boot

1. Insert the card and boot the RG34XX. BaseOS `frontend-session` should select
   `System/launch_frontend.sh` and enter PocketJS.
2. Confirm the display is stable at the intended orientation and the whole UI
   fits the panel. Confirm controls respond: L/R change workspaces, directions
   move selection, A activates, B returns.
3. Confirm the top status bar and five workspaces: HOME, MEDIA, GAMES, SETUP,
   SYSTEM. Confirm the horizontal card carousel and selected-card highlight.
4. Confirm Library limitations are understood: this stage has no game scan and
   cannot launch child `.pocket` packages from Library.
5. Read `/tmp/pocketjs-frontend.log` over a shell session and confirm it records
   package path, admission output, DRM card node, and runtime exit code if it
   exits. It is a RAM log and is lost on power-off.

## Admission and DRM troubleshooting

On the device console, from the SD root, run:

```sh
System/pocketjs/bin/pocket-linux-kms --package System/pocketjs/apps/boot.pocket --check
```

Expected shape:

```text
package admitted: System/pocketjs/apps/boot.pocket (rocknix-h700-kms)
```

The exact output and success on the target are **not verified**. If admission
fails (especially `ProfileHashMismatch`), stop and retain the log; do not loop
launch attempts or replace the package without rebuilding against the target
profile. Check which DRM node is present with `ls -l /dev/dri/card0 /dev/dri/card1`
and compare with the frontend log.

## SSH and ADB defaults

BaseOS README documents SSH/SFTP over Wi-Fi and ADB over USB as enabled by
default. Verify on first boot:

- SSH: connect to the hostname/IP configured by BaseOS. The default SSH user is
  `root`; use the password configured in `baseos.conf` or BaseOS defaults. Do not
  publish credentials. Check that a shell can read `/tmp/pocketjs-frontend.log`.
- ADB: connect a data-capable USB cable before powering on, then run `adb
  devices` on the host computer and confirm the handheld is listed.
- BaseOS settings: inspect/edit `baseos.conf` at the card root to configure
  `hostname` and `mdns`; reboot and confirm the hostname resolves on the LAN.

SSH reachability, ADB enumeration, host name resolution, and frontend operation
are all **not verified** on the target by this workspace task.
## M1 runtime load record (2026-10-30)

Build command: `./tools/build-runtime.sh` (ARM64 Alpine, pinned PocketJS
`e6770376`, Rust 1.93.0, `aarch64-unknown-linux-musl`, ALSA disabled,
rocknix-h700 profile exported with `bun tools/pocket.ts profile export`).

Host ELF summary:

```text
$ file inputs/pocket-linux-kms
ELF 64-bit LSB executable, ARM aarch64, statically linked, not stripped
$ readelf -V inputs/pocket-linux-kms
(no version information found)
$ readelf -l inputs/pocket-linux-kms | grep INTERP
(no output)
$ sha256sum inputs/pocket-linux-kms
035be28e3cbc05724350dac9a0ca95ab65d33022403138692f77d001728719d6
```

Device acceptance (binary copied through hx90, then `adb push`):

```text
$ adb -s 5c0010894414c802512 shell \
    "/mnt/sdcard/System/pocketjs/bin/pocket-linux-kms \
     --package /mnt/sdcard/System/pocketjs/apps/boot.pocket \
     --headless --frames 60"
completed 60 logical ticks; 720x480; rocknix-h700-kms
```

No loader or `GLIBC_*` error occurred. M1 complete.


## M2 display and controls record (2026-10-30)

Direct fbdev run (no DRM present):

```text
$ adb -s 5c0010894414c802512 shell \
    "/mnt/sdcard/System/pocketjs/bin/pocket-linux-kms \
     --package /mnt/sdcard/System/pocketjs/apps/boot.pocket \
     --fb /dev/fb0 --input /dev/input/event1"
DRM unavailable (No such file or directory (os error 2)); trying framebuffer
display: fbdev (timer pacing)
completed 611 logical ticks; 720x480; rocknix-h700-kms
```

Frames captured with `dd if=/dev/fb0 ...` and converted from the 720×960
virtual mapping to 720×480 PNG:

- `docs/evidence/m2-before.png`: HOME workspace, Terminal card selected.
- `docs/evidence/m2-after-L.png`: L moved to SYSTEM (WS 5/5).
- `docs/evidence/m2-after-R.png`: R returned to HOME (WS 1/5).
- `docs/evidence/m2-after-A.png`: A opened the Terminal.
- `docs/evidence/m2-after-B.png`: B returned to HOME.

Input mapping used: A=305 (CIRCLE), B=304 (CROSS), L=310, R=311.
Full commands, helper build, and frame notes: `docs/evidence/M2-display.txt`.
M2 complete.

## Recovery


If the app exits or admission fails, BaseOS init respawns the script after its
throttling interval. After three consecutive observed runtime failures the
script waits 30 seconds before returning. Use SSH or ADB to inspect the
RAM-resident log while the device remains powered. For rollback, restore the
previous `System/` tree from the `.pocketjs-backup-*` directory on the card.
