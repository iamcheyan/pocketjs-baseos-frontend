#!/usr/bin/env python3
"""Select the pinned PocketJS Linux KMS workspace and its dedicated lock."""
from pathlib import Path
import re
import shutil
import sys

root = Path(sys.argv[1])
manifest = root / "Cargo.toml"
lock = root / "Cargo.lock"
for path in (manifest, lock):
    backup = path.with_name(path.name + ".upstream")
    if not backup.exists():
        shutil.copy2(path, backup)

text = manifest.with_name("Cargo.toml.upstream").read_text()
for key in ("members", "default-members"):
    text, count = re.subn(
        r"(?ms)^" + key + r" = \[.*?^\]",
        key + ' = ["programs/linux-kms"]',
        text,
        count=1,
    )
    if count != 1:
        raise SystemExit(f"workspace has no unique {key} declaration")
manifest.write_text(text)

# libc's ioctl request parameter is target-dependent (c_int on musl/aarch64,
# c_ulong on glibc). `as _` adapts the same ioctl word to the target signature.
native = root / "platforms/linux/evdev/src/native.rs"
source = native.read_text()
for old, new in (
    ("request(0x20 + kind, bytes.len()),", "request(0x20 + kind, bytes.len()) as _,"),
    ("request(0x40 + code, size_of::<AbsInfo>()),", "request(0x40 + code, size_of::<AbsInfo>()) as _,"),
):
    if new not in source:
        if old not in source:
            raise SystemExit(f"evdev ioctl anchor missing: {old!r}")
        source = source.replace(old, new, 1)
native.write_text(source)

runtime_lock = Path("/workspace/tools/pocketjs-linux-kms.Cargo.lock")
if runtime_lock.is_file():
    shutil.copy2(runtime_lock, lock)
else:
    shutil.copy2(lock.with_name("Cargo.lock.upstream"), lock)
print("Linux KMS workspace selected; preparing its dedicated lockfile")
