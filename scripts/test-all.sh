#!/bin/bash
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed -E 's/version *= *"([^"]+)"/\1/')
VERSION="v${VERSION}"

echo "version: ${VERSION}"

cargo fmt --all --check && cargo clippy -- -D warnings

targets=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

profile="release"
for target in "${targets[@]}"; do
    echo "==> Testing for $target"
    cross test --profile $profile --target "$target"
done

