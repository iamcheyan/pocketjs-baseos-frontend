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

## Blocked / open questions
- **The PocketJS-native Solid launcher bundle could not be built here.** `inputs/launcher-src/`
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
