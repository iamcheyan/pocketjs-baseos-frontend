# PocketJS frontend migration to BaseOS

## Stage 1: bootable frontend payload

The payload installs `System/launch_frontend.sh`, the PocketJS Linux KMS runtime,
and the approved launcher package. BaseOS `frontend-session` invokes the
script from the SD root through `/bin/sh`; no systemd unit is used. The script
checks runtime and package presence, selects DRM when a card node exists and
otherwise falls back to `--fb /dev/fb0`, chooses the `ANBERNIC-keys` evdev node
when present, runs package admission with `--check`, then starts the runtime.
If the primary package is rejected it retries `boot.approved.pocket`.
Diagnostics stay in `/tmp/pocketjs-frontend.log` (RAM). Consecutive startup or
runtime failures back off, reaching 30 seconds from the third failure.

The approved `inputs/boot.pocket` baseline is 4,084,728 bytes with SHA-256
`d2836f45e0221dd0de1525a38324cc56f0915282fb864cc74570e707dfa93ee2`. It carries
the HOME / MEDIA / GAMES / SETUP / SYSTEM workspaces, the top Wi-Fi/Bluetooth/
IPv4/WS/battery status bar, horizontal `CardCarousel`, and L/R workspace
navigation with directional selection, A launch, and B back. The package profile
is `rocknix-h700-kms`; admission on the target device is a required check.

## Runtime API findings

The SDK filesystem module is not an unrestricted Node filesystem API.
`framework/src/fs-api.ts` describes paths as relative to an app's own data root
(lines 17–20); `contracts/spec/fs.ts` specifies that the host binds this root
and the guest cannot name absolute paths or another app's tree (lines 27–37).
The module throws when `globalThis.fs` is not mounted (fs-api.ts lines 23–27,
67–72). The current Linux KMS target advertises only `input.buttons`,
`input.pointer`, `input.touch`, and `text.glyphs.baked` in
`programs/linux-kms-h700/pocket.runtime-config.ts` lines 7–18, and its H700
profile likewise has no `data.fs`. Its runtime loads one package and boots one
guest (`programs/linux-kms/src/main.rs` lines 138–151 and 239–258); the inspected
host does not expose a guest filesystem mount or arbitrary host filesystem API.
The approved launcher manifest also does not request `data.fs`.

**Conclusion:** this runtime cannot use the PocketJS SDK `readdirSync` API to
scan `/mnt/sdcard/games/`. The first-stage payload therefore starts the approved
launcher UI, with Library discovery unavailable. No Python or shell scanner is
used to bypass the sandbox.

## Library and child package limitations

The approved launcher baseline has fallback behavior for missing host bridge
calls (`host?.__pocketshell?.(command) ?? ""`). This lets its UI render without
the ROCKNIX bridge; it does not provide file enumeration or package execution.
The Library data list remains empty and its content/document functions have no
BaseOS data provider. The selected package cannot launch another `.pocket` from
inside the current Linux KMS runtime.

Evidence for package switching:

- `programs/linux-kms/src/main.rs` accepts one package at process start and
  boots/ticks that guest; no app-switch operation provider is registered.
- `framework/src/launcher.ts` treats `appTable` and `appLaunch` as optional host
  operations; absent operations return null/false.
- The compiled bridge's `ROMLAUNCH` path calls `systemd-run` with
  `pocketshell-play`; a BaseOS shim must preserve frontend DRM release and init
  respawn. The supplied runtime strings alone do not prove this workflow works.

No game launch is reported as supported until the compatibility commands are
implemented and the launch/return flow passes on device.

## Stage 2: existing PocketShell bridge compatibility layer

The supplied runtime binary contains PocketShell bridge strings, including
`__pocketshell`, `__launcher`, `POCKETSHELL_LIBRARY_ROOTS`,
`POCKETSHELL_LIBRARY_DATA_ROOT`, `/usr/bin/pocketshell-games`,
`/usr/bin/pocketshell-play`, `systemd-run --unit=pocketshell-game`,
`systemctl`, `pactl`, and `amixer`. This is direct `strings` evidence that the
bridge code was compiled into the binary; it does **not** prove its commands
work on BaseOS or that every shim path is present.

The revised plan reuses that bridge and supplies host compatibility pieces.
No new ABI or runtime rebuild is planned unless device checks show the existing
binary cannot reach the bridge operations.

**Compatibility scope:** the frontend currently only attempts the `/storage` bind.
The candidate games indexer is isolated in `tools/build-compatibility-layer.sh`
and is not in `System/` or the SD payload. It must be installed into BaseOS's
executable search path by a separately reviewed overlay before `ROMSCAN` can use
it. `POCKETSHELL_LIBRARY_ROOTS` belongs to the document reader bridge; point it
at games only if the product accepts the reader also seeing those files.

1. **Map persistent paths.** The frontend entry bind-mounts the SD root on the
   existing `/storage` mount point when available. This makes the bridge's
   existing absolute paths resolve on the card. Confirm the target image has
   `/storage`, `mountpoint`, and permission for `mount --bind`; the script logs
   success/failure. A failed bind leaves the launcher able to start but does
   not provide working persistent bridge data.
2. **Library roots and games index are separate.** `POCKETSHELL_LIBRARY_ROOTS` and
   `POCKETSHELL_LIBRARY_DATA_ROOT` configure the document reader bridge; upstream
   `pocketshell-library.rs` confirms this. They do not bypass the games scanner.
   The games bridge's `ROMSCAN` executes `/usr/bin/pocketshell-games scan` and
   returns stdout; the upstream scanner writes `sections.games` to
   `/storage/pocketshell/library/library.json`.
3. **Games scanner shim.** BaseOS lacks Python 3, so implement a BusyBox shell
   scanner with the same output schema, stable grouping/sorting, and supported
   extensions. The draft generator has passed an isolated fixture check but is
   not installed or target-verified.
3. **Game launch.** The bridge references `systemd-run` and
   `/usr/bin/pocketshell-play`; BaseOS has no systemd. Do not provide a generic
   background `systemd-run` shim: the current runtime owns DRM and a child game
   must receive display/input ownership safely. Implement a narrowly scoped
   `systemd-run` compatibility wrapper plus a BaseOS `pocketshell-play` path
   only after tracing the actual bridge arguments and proving the sequence
   launch → game exit → frontend respawn. Until then Library launch remains
   disabled/unavailable.
4. **Settings and services.** Inventory BaseOS's `systemctl` shim and extend
   only the exact bridge actions needed to report default-on `dropbear`/`adbd`
   and supported daemons. Check `pactl` and `amixer` availability; adapt volume
   calls to the installed ALSA interface without claiming missing services
   exist. Bind the settings data path under the card and create defaults only
   where the bridge expects them.
5. Keep the source snapshot `inputs/launcher-src/` and the full
   `inputs/CARD-BASELINE.patch`; retain the horizontal 104×108 CardCarousel,
   icon and selected styling, five workspace navigation, and status bar. Any
   BaseOS-specific UI change must be listed in the migration record.

The three-stage scope and status table are in [ROADMAP.md](ROADMAP.md). Stage 2
is a compatibility-layer task, not a new-ABI project; its Library and game
launch acceptance remains unverified until the bridge shims are implemented and
exercised on BaseOS hardware.

## Compatibility-layer implementation status

The current entry script performs a best-effort SD-root bind mount at
`/storage`. `tools/build-compatibility-layer.sh` stages a draft
`pocketshell-games` JSON generator and service-status helper under ignored
`dist/compat/`; it does not install them into BaseOS or claim device support.
The generator follows the old bridge's `{"sections":{"games":[...]} ,"total":N}`
catalogue contract and passed a small isolated fixture check.

The upstream `pocketshell-play` invokes `pocket-session-run`, which owns display
and service handover. A direct child runtime launch while the frontend still
holds DRM is unsafe; game launch remains a later task after ownership and init
respawn are proven on target hardware.

Stage 2 status: **not complete**. See [ROADMAP.md](ROADMAP.md). BaseOS
systemctl integration, scanner target validation, audio semantics, and game
launch/return remain unverified.

## Source and package baseline

- Source snapshot: `inputs/launcher-src/` (feat/pocketshell-cards approved
  snapshot); `inputs/CARD-BASELINE.patch` records its 1,099-line uncommitted
  change set. Do not use `inputs/launcher-src-WRONG-grid-old/`.
- UI reference package: `inputs/boot.pocket`, size 4,084,728 bytes,
  SHA-256 `d2836f45e0221dd0de1525a38324cc56f0915282fb864cc74570e707dfa93ee2`.
- Runtime: rebuilt from the pinned source with `tools/build-runtime.sh`; target
  device loading remains unverified until M1 acceptance succeeds.
- `boot.pocket` is a package artifact, not a substitute for the source snapshot.

## Verification record

Input checks run:

- `stat -c '%n %s bytes %A' inputs/boot.pocket inputs/pocket-linux-kms` →
  `inputs/boot.pocket 4084728 bytes -rw-r--r--`; runtime `2233080 bytes -rwxr-xr-x`.
- `sha256sum inputs/boot.pocket` →
  `d2836f45e0221dd0de1525a38324cc56f0915282fb864cc74570e707dfa93ee2`.

Acceptance commands and actual output:

- `sh -n System/launch_frontend.sh` → exit 0.
- `busybox sh -n System/launch_frontend.sh` → exit 0.
- `tools/verify-payload.sh` → `verify-payload: PASS`; runtime input 2,233,080 bytes
  executable; package input 4,084,728 bytes; shell syntax passed; forbidden
  launcher dependencies none.
- `tools/make-payload.sh` → `dist/sdcard-payload.tar.gz`, 1,358,897 bytes,
  SHA-256 `8f0915a8d91ad9a88d8894f46a48d4670199db8b704e0ea495e978267488ae0d`.
- `tar -tzf dist/sdcard-payload.tar.gz` → only `System/` and `README.txt` trees.
- `tools/build-compatibility-layer.sh` plus isolated scanner fixture →
  `scanner fixture: PASS total=3 groups=gba,snes,misc`.
- `tools/deploy-to-card.sh <temporary-directory>` → `deploy smoke: PASS`; old
  System/ and README.txt were preserved in the timestamped backup.

Target package admission, BaseOS boot, display/input, bridge shim activation,
Library behavior, SSH/ADB reachability, and game launch/return remain **not
verified**; complete the device checklist before treating Stage 1/2 as accepted.
## BaseOS musl runtime build

`tools/build-runtime.sh` checks out pinned PocketJS revision
`e6770376626a4de5c93eaa6fb048e7e9ffb81f9f` into ignored `.build/pocketjs/`.
It copies the `rocknix-h700-kms` profile from `tools/pocket-runtime-profile/`
into the source tree, exports `pocket_profile.rs` with Bun, and builds
`pocket-linux-kms` natively in an ARM64 Alpine container with Rust 1.93.0 and
target `aarch64-unknown-linux-musl`. The build disables ALSA and installs the
static executable at `inputs/pocket-linux-kms`. `prepare-runtime-workspace.py`
also applies the target-adaptive `ioctl` request cast required by the musl
`libc` signature. Run the script after changing the pin or the recipe; do not
commit the upstream checkout.

The pinned PocketJS source declares `rust-version = "1.93"`; its toolchain file
selects a newer Rust release. The BaseOS build intentionally uses Rust 1.93.0,
the declared minimum, through `cargo +1.93.0`.

M1 device acceptance is recorded in `docs/DEVICE-CHECKLIST.md`: the static
binary completed 60 logical ticks on the RG34XX with the `rocknix-h700-kms`
package and no loader error.
