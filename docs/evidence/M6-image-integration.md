# M6 — single-image integration (status)

Integration completed in the BaseOS source tree (`~/development/PJOS-LINUX`):

- `overlay/usr/share/pocketshell/System` carries the built payload
  (`launch_frontend.sh`, `pocketjs/{bin,apps,version}`, `compat/`) so the
  rootfs bakes it at `/usr/share/pocketshell/System`.
- `overlay/usr/sbin/expand-storage` now, after `mkfs.vfat` on a freshly grown
  p7, copies `/usr/share/pocketshell/System` to the card and logs
  `storage expanded; baked PocketJS System/ copied to the card`. This runs in
  `rcS` before `frontend-session`, matching the M6 plan (方案 A).

Remaining to satisfy M6 acceptance:

1. Rebuild the BaseOS image:
   `./build-rootfs.sh rg34xx && ./build-image.sh rg34xx`
   (the PJOS repo's commit was held back by its local privacy-identity hook;
   the overlay payload and expand-storage edit are present in the worktree).
2. Flash an empty card with `./flash-card.sh` (or an external flasher).
3. First boot: the freshly formatted p7 should receive `System/` and the
   device should boot straight into the launcher.

Evidence for the first-boot autostart on a **pre-populated** card already
exists in `docs/evidence/m3-boot.*`; M6 needs the equivalent capture from a
genuinely empty flashed card.

Status: M6 not complete — pending image rebuild and physical card flash.
