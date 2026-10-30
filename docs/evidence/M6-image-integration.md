# M6 — single-image integration (status)

Integration completed in the BaseOS source tree (`~/development/PJOS-LINUX`):

- `overlay/usr/share/pocketshell/System` carries the built payload
  (`launch_frontend.sh`, `pocketjs/{bin,apps,version}`, `compat/`) so the
  rootfs bakes it at `/usr/share/pocketshell/System`.
- `overlay/usr/sbin/expand-storage` now, after `mkfs.vfat` on a freshly grown
  p7, copies `/usr/share/pocketshell/System` to the card and logs
  `storage expanded; baked PocketJS System/ copied to the card`. This runs in
  `rcS` before `frontend-session`, matching the M6 plan (方案 A).

The first-boot sync step is verified in the BaseOS regression harness:
`./test-expand-storage.sh` passes, including `ok: baked PocketJS System/ is
copied on first boot` (asserts `launch_frontend.sh`, `pocket-linux-kms` and the
compat scanner land on the freshly formatted p7). See
`docs/evidence/M6-expand-sync.txt`.

Remaining to satisfy M6 acceptance:

1. Rebuild the BaseOS image:
   `./build-rootfs.sh rg34xx && ./build-image.sh rg34xx`
   (the PJOS repo's commit was held back by its local privacy-identity hook;
   the overlay payload and expand-storage edit are present in the worktree;
   `work/tools` only has `busybox` and `dropbearmulti` built so `build-tools.sh`
   must run first).
2. Flash an empty card with `./flash-card.sh` (or an external flasher).
3. First boot: the freshly formatted p7 should receive `System/` and the
   device should boot straight into the launcher.

Evidence for the first-boot autostart on a **pre-populated** card already
exists in `docs/evidence/m3-boot.*`; M6 needs the equivalent capture from a
genuinely empty flashed card.

Status: M6 not complete — pending full toolchain image build and physical
card flash.

