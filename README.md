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

## Current limitations

The launcher UI and card navigation are present. BaseOS's current PocketJS
Linux KMS runtime does not provide guest filesystem access to
`/mnt/sdcard/games/` or app switching, so Library scanning and child package
launch are unavailable in this stage. Details and the second-stage native
bridge proposal are in [docs/MIGRATION.md](docs/MIGRATION.md). Hardware steps
are in [docs/DEVICE-CHECKLIST.md](docs/DEVICE-CHECKLIST.md).
