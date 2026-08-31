#!/usr/bin/env bash

# usage: ./web/build-web.sh [--dev] [--serve]

set -euo pipefail

cd "$(dirname "$0")/.."

profile=dist
serve=0

for arg in "$@"; do
    case "$arg" in
        --dev) profile=dev ;;
        --serve) serve=1 ;;
        *) echo "usage: $(basename "$0") [--dev] [--serve]" >&2; exit 1 ;;
    esac
done

case "$profile" in
    dev) artifact_dir=debug ;;
    *) artifact_dir="$profile" ;;
esac

wasm_bindgen_version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')

rustup target add wasm32-unknown-unknown

if ! cargo install --list | grep -q "^wasm-bindgen-cli v$wasm_bindgen_version:"; then
    cargo install --force --quiet wasm-bindgen-cli --version "$wasm_bindgen_version" --locked
fi

rm -f web/pkg/rau_bg.wasm

cargo build --profile "$profile" --lib --target wasm32-unknown-unknown

wasm-bindgen \
    --target web \
    --out-dir web/pkg \
    --out-name rau \
    --no-typescript \
    "target/wasm32-unknown-unknown/$artifact_dir/rau.wasm"

if [ "$profile" != dev ]; then
    if command -v wasm-opt >/dev/null 2>&1; then
        wasm-opt web/pkg/rau_bg.wasm -O2 -o web/pkg/rau_bg.wasm
    else
        echo "warning: wasm-opt not found, shipping unoptimized wasm" >&2
    fi
fi

echo "built web/pkg from the $profile profile ($(stat -c %s web/pkg/rau_bg.wasm) bytes)"

if [ "$serve" = 1 ]; then
    npx --yes serve web
fi
