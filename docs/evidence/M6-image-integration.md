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

Image build completed:

```text
./build-tools.sh rg34xx          # all static aarch64 tools
./build-rootfs.sh rg34xx         # 117M rootfs.tar, payload present
./build-image.sh rg34xx          # 1.4G image, checks passed
Image: ~/development/PJOS-LINUX/work/rg34xx/baseos-rg34xx.img
```

Verified in the built rootfs:
`usr/share/pocketshell/System/{launch_frontend.sh,pocketjs/bin/pocket-linux-kms,compat/bin/pocketshell-games}`
are present. Two BaseOS build fixes were needed and are in the worktree:
the h700-toolchain container lacked `curl` (mount the built static curl) and
the injected PATH must keep `/opt/aarch64-nextui-linux-gnu/bin`.

Remaining to satisfy M6 acceptance:

1. Flash an empty card with `./flash-card.sh rg34xx <disk>`.
2. First boot: the freshly formatted p7 should receive `System/` and the
   device should boot straight into the launcher; capture the first-boot frame.

Evidence for the first-boot autostart on a **pre-populated** card already
exists in `docs/evidence/m3-boot.*`; M6 needs the equivalent capture from a
genuinely empty flashed card.

## Integrated launcher package correction

The initial image contained the old 4,084,728-byte package. The payload now
uses the integrated 5,599,224-byte package as `boot.pocket`, retaining the old
package as `boot.approved.pocket` for launcher fallback.

```text
boot.pocket          a8451b887ce24f203ee85b3f7f9dfe93d5e743b963a4c90eb9969f56688ea9e4
boot.approved.pocket d2836f45e0221dd0de1525a38324cc56f0915282fb864cc74570e707dfa93ee2
image                a7e06eca0ae38e845b982bc5ff383a2c8085b19fd9a9e75aaa3b6a9c31c1c2f5
```

Both package hashes were verified from `work/rg34xx/rootfs.tar` after the
rebuilt rootfs and image completed successfully. Physical first boot remains
for manual acceptance when an SD card is available; it has not been claimed
or simulated.
