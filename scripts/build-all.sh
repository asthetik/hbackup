#!/bin/bash
set -e

targets=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

for target in "${targets[@]}"; do
    echo "==> Building for $target"
    cross build --release --target "$target"
done
