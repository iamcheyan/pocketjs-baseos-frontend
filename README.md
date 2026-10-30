# PocketJS BaseOS frontend payload

First-stage payload for the Anbernic RG34XX running BaseOS. It boots the
approved horizontal-card PocketJS launcher through BaseOS `frontend-session`.

## Build the SD payload

```sh
./tools/make-payload.sh
```

This validates the tree and creates `dist/sdcard-payload.tar.gz`, containing
only `System/` and `README.txt`.

## Deploy to a mounted card

```sh
./tools/deploy-to-card.sh /path/to/mounted/sd-root
```

The script backs up any existing `System/` and `README.txt` before extraction.
Keep that backup until the handheld has passed the device checklist.

## Preview and screenshots

The RG34XX launcher preview is a dependency-free static page:

```sh
cd inputs/launcher-src && python3 -m http.server 8099 --bind 127.0.0.1
# open http://127.0.0.1:8099/browser-preview.html
```

`CHROME_BIN=/usr/bin/google-chrome node tools/capture-screenshots.mjs \
  http://127.0.0.1:8099/browser-preview.html docs/screenshots` captures the 720×480 panel in
seven states (apps/services/settings, dark/light, selection, service toggle) and writes a
manifest with the run timestamp and SHA-256 of each PNG. See `VERIFICATION.md`.

## Current limitations

The launcher UI and horizontal card navigation are present. BaseOS has no guest
filesystem bridge to enumerate `/mnt/sdcard/games/`, and Linux KMS cannot switch
to another package during the current runtime process. The compiled PocketShell
bridge requires compatibility shims; its staged scanner draft is not installed
or device-verified. See [docs/MIGRATION.md](docs/MIGRATION.md),
[docs/ROADMAP.md](docs/ROADMAP.md), and
[docs/DEVICE-CHECKLIST.md](docs/DEVICE-CHECKLIST.md).
