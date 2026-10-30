# Verification — pocketjs-baseos-frontend @ 2026-10-31

- Verdict: PARTIAL
- Commit: `6167df764c8315374f6b12104cb89e2796175efe` (preview capture + docs; this document is the following commit)
- Remote: `6167df764c8315374f6b12104cb89e2796175efe (origin/master; this repo has no main)`
- Runtime: SD payload build (`tools/make-payload.sh`) + static RG34XX launcher preview (`inputs/launcher-src/browser-preview.html`) served on `127.0.0.1:8099` and driven in headless Chrome
- Toolchain: sh/busybox shell checks, node v22.23.3, Chrome 155.0.8059.84

## Test gate
| command | result | notes |
|---|---|---|
| `tools/verify-payload.sh` | PASS | runtime + boot.pocket inputs present/executable; `sh -n` and busybox `ash -n` pass; no systemctl/python//storage/ dependency in the launcher script |
| `tools/make-payload.sh` | PASS | builds `dist/sdcard-payload.tar.gz` (1,358,897 bytes, sha256 `3a0bfb6c…fa5bf`) containing only `System/` + `README.txt` |
| payload inspection | PASS | `tar -tzf` lists `System/launch_frontend.sh`, `System/pocketjs/apps/boot.pocket`, `System/pocketjs/bin/pocket-linux-kms`, `System/pocketjs/version`, `README.txt` |
| `node tools/capture-screenshots.mjs` | PASS | 7/7 preview states, 7 unique hashes, no overflow, no blank frames |

## Screenshot matrix
The preview emulates the 720×480 RG34XX panel with the real app/service/settings lists; the
script clicks/keys the same controls a user would and clips the `#screen` element, asserting
that the rendered section and selected row match the requested navigation.

| file | viewport | path | rendered state |
|---|---|---|---|
| apps-dark-720x480.png | 720×480 | `docs/screenshots/apps-dark-720x480.png` | 应用, selected “▶文件管理器”, dark |
| apps-light-720x480.png | 720×480 | `docs/screenshots/apps-light-720x480.png` | 应用, selected “▶文件管理器”, light |
| apps-selected-720x480.png | 720×480 | `docs/screenshots/apps-selected-720x480.png` | 应用, selected “▶存储信息”, dark |
| services-dark-720x480.png | 720×480 | `docs/screenshots/services-dark-720x480.png` | 服务, selected “▶ADBD开启”, dark |
| services-toggled-720x480.png | 720×480 | `docs/screenshots/services-toggled-720x480.png` | 服务, selected “▶ADBD关闭”, dark |
| settings-dark-720x480.png | 720×480 | `docs/screenshots/settings-dark-720x480.png` | 设置, selected “▶主题：深色”, dark |
| settings-light-720x480.png | 720×480 | `docs/screenshots/settings-light-720x480.png` | 设置, selected “▶主题：浅色”, light |

Count: 7 screenshots, 7 unique hashes (0 duplicates). `tools/check_shots.py`: valid PNGs,
720×480 each, minimum RGB stddev 41.74, no blank frames. `docs/screenshots/manifest.json`
records the timestamp, URL, Chrome path and per-file SHA-256.

## Fixed / added
- There was no way to run or capture the launcher UI from this repository: the only shipped UI is
  `inputs/launcher-src/browser-preview.html`, and no screenshot harness existed. → added
  `tools/capture-screenshots.mjs` (dependency-free CDP driver: apps/services/settings lists,
  dark/light, selection and service-toggle states, 720×480 clip, hash/overflow/blank gates,
  manifest) and documented the preview flow in `README.md` → 7 captures above.
- The payload build produces the SD tarball with only `System/` + `README.txt`, verified by
  listing the archive.

## 2026-10-31 05:05 — native bundle rebuilt and sim-verified

The launcher bundle below is a **new** compile of `inputs/launcher-src` (the same source
committed in `19f12b1`), not the 2026-10-29 baseline.

- Build host: HX90 (aarch64, NixOS); PocketJS SDK `~/development/pjs-app-build`
- Command (run in that SDK):
  `bun tools/pocket.ts compile --manifest apps/rocknix-h700/pocket.json --runtime-config programs/rocknix-h700/pocket.runtime-config.ts --target rocknix-h700-kms --output boot.pocket`
- Output: `inputs/boot.pocket` — 4,649,304 bytes, sha256 `ea7cc0ce882633089d0e928977ee5c807519a0b539ac930ee4bd9caba1bb9826`
- Plan: `sha256:eea89226e55a1b3bb16e9f9b152e501e0c623c3f35dcecf2b51d3e67a85ffed4`; target `rocknix-h700-kms`,
  720×480 physical / 360×240 logical, raster 2x; compiler reported 78 app modules, 9 pak entries, 211 declared characters.
- Supersedes the 2026-10-29 baseline `d2836f45…` (4,084,728 bytes). Keep it as the rollback copy.
- Runtime aligned with the 1.5.3 image: `inputs/pocket-linux-kms` is now the static musl build baked into
  that image's rootfs (3,118,712 bytes, sha256 `20f7d0d7…`), not the older 2,233,080-byte dynamically linked
  build, which does not even execute outside the target (missing `/lib/ld-linux-aarch64.so.1`).

**Bug found and fixed while verifying:** `pages/settings-page.tsx` `statePill()` built its class string with a
nested template literal (`` `text-xs font-bold ${color === "green" ? … : …}` ``). PocketJS resolves classes
against a compile-time table, so opening SETTINGS at runtime threw
`unknown class "text-xs font-bold text-sky-300" - not in the compiled style table` and the panel failed to
render. Rewritten as a ternary of full literals; the panel now mounts.

**Sim verification** (`programs/sim/sim.ts` `runScenario`, 60 Hz, virtual frames only — no device involved):

| state | result |
|---|---|
| boot (APPS) | renders tab bar APPS/SETTINGS/SYSTEM/TOOLS + status bar + app list |
| `RTRIGGER` → SETTINGS | renders 16 rows, selection, progress bars, ON/OFF pills |
| `RTRIGGER` ×2 → SYSTEM | renders OVERVIEW panel (CPU/MEM/STORAGE placeholders, no host data in sim) |
| `DOWN`×2 + `CIRCLE` on Wi-Fi row | renders Wi-Fi page, "正在扫描… / 未发现网络" (sim has no `system.wifi` host) |

Frames: `docs/screenshots/sim/01-apps.png` … `04-wifi.png` (480×272 sim framebuffer, not a device capture).

**Package admission (the check `launch_frontend.sh` runs before starting the runtime):**

```
$ System/pocketjs/bin/pocket-linux-kms --package System/pocketjs/apps/boot.pocket --check
package admitted: …/boot.pocket (rocknix-h700-kms)   # exit 0
```

run against the exact files extracted from the payload tarball, on the aarch64 build host.

**Rebuilt payload:** `dist/sdcard-payload.tar.gz` — 1,875,521 bytes,
sha256 `dafa73c373717acdbfd865dc603e40f056e913d8d93482e4901ba0489d56c529`, containing
`System/pocketjs/apps/boot.pocket` (`ea7cc0ce…`) and `System/pocketjs/bin/pocket-linux-kms` (`20f7d0d7…`).

Still unverified: everything on real RG34XX hardware — the device did not answer on its LAN address
during this round and no SD card was present in the HX90 reader, so package admission, KMS presentation
and the boot path remain `docs/DEVICE-CHECKLIST.md` items.

## Blocked / open questions
- **Resolved 2026-10-31:** the bundle is now rebuilt and sim-verified (previous round's note kept below). `inputs/launcher-src/`
  contains `main.tsx`/`library.tsx`/`showcase/*` that need the pinned PocketJS compiler
  (`pocket compile`, WASM core/rasterizer) which is not vendored in this repository; the
  showcase capture script even reads `/tmp/pocketjs-showcase-*` build artefacts. So the native
  `boot.pocket` UI was not recompiled or run on-device in this round; only the payload inputs
  and the browser preview are verified. This is why the verdict is PARTIAL, not PASS.
- No RG34XX hardware was attached: KMS/evdev behaviour, boot from SD and the PocketShell bridge
  remain device-checklist items (`docs/DEVICE-CHECKLIST.md`).

## Not verified here
- 掌机真机（RG34XX / PJOS-LINUX）未验证；本轮只在本机 headless Chrome 里跑真实进程。
- Real PocketJS runtime packaging (`boot.pocket` recompile), KMS presentation, and the native
  file-system bridge.
