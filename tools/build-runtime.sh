#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SOURCE="$ROOT/.build/pocketjs"
PIN=e6770376626a4de5c93eaa6fb048e7e9ffb81f9f
IMAGE=rust:1.93-alpine@sha256:4fec02de605563c297c78a31064c8335bc004fa2b0bf406b1b99441da64e2d2d

[ -d "$SOURCE/.git" ] || {
    mkdir -p "$ROOT/.build"
    git clone https://github.com/pocket-nexus/pocketjs.git "$SOURCE"
}
git -C "$SOURCE" checkout "$PIN"
[ "$(git -C "$SOURCE" rev-parse HEAD)" = "$PIN" ] || {
    echo "unexpected PocketJS revision" >&2
    exit 1
}

mkdir -p "$SOURCE/programs/rocknix-h700" "$ROOT/.build/profile-export"
cp "$ROOT/tools/pocket-runtime-profile/profile.ts" \
   "$ROOT/tools/pocket-runtime-profile/pocket.runtime-config.ts" \
   "$SOURCE/programs/rocknix-h700/"
command -v bun >/dev/null 2>&1 || {
    echo "bun is required to export the runtime profile" >&2
    exit 1
}
[ -d "$SOURCE/node_modules" ] || (
    cd "$SOURCE" && bun install --frozen-lockfile --ignore-scripts
)
[ -f "$ROOT/.build/profile-export/pocket_profile.rs" ] || (
    cd "$SOURCE" && bun tools/pocket.ts profile export \
        --config programs/rocknix-h700/pocket.runtime-config.ts \
        --out "$ROOT/.build/profile-export"
)
exec docker run --rm --platform linux/arm64 \
    -v "$ROOT:/workspace" -w /workspace/.build/pocketjs \
    -e POCKETJS_PROFILE_RS=/workspace/.build/profile-export/pocket_profile.rs \
    "$IMAGE" sh -lc '
        set -eu
        apk add --no-cache curl musl-dev clang llvm-dev pkgconf build-base
        curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
          | sh -s -- -y --profile minimal --default-toolchain 1.93.0 \
              --target aarch64-unknown-linux-musl
        . /usr/local/cargo/env
        python3 /workspace/tools/prepare-runtime-workspace.py /workspace/.build/pocketjs
        if [ -f /workspace/tools/pocketjs-linux-kms.Cargo.lock ]; then
          cp /workspace/tools/pocketjs-linux-kms.Cargo.lock Cargo.lock
        else
          cargo +1.93.0 generate-lockfile
          cp Cargo.lock /workspace/tools/pocketjs-linux-kms.Cargo.lock
        fi
        cargo +1.93.0 build --locked --release \
          --target aarch64-unknown-linux-musl \
          --package pocket-linux-kms --no-default-features
        install -D -m 0755 \
          target/aarch64-unknown-linux-musl/release/pocket-linux-kms \
          /workspace/inputs/pocket-linux-kms
    '
