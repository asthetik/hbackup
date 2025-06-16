#!/bin/bash
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed -E 's/version *= *"([^"]+)"/\1/')
VERSION="v${VERSION}"

echo "version: ${VERSION}"

targets=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

for target in "${targets[@]}"; do
  if [ -e "dist/${target}" ]; then
    rm -r "dist/${target}"
  fi
done

for file in dist/bk-v*.tar.gz; do
  if [ -e "$file" ]; then
    rm "$file"
  fi
done

for file in dist/bk-v*.zip; do
  if [ -e "$file" ]; then
    rm "$file"
  fi
done

for target in "${targets[@]}"; do
    echo "==> Building for $target"
    cross build --release --target "$target"

    # Prepare output directory
    out_dir="dist/$target"
    mkdir -p "$out_dir"

    # Determine binary name
    bin_name="bk"
    if [[ "$target" == *windows* ]]; then
        bin_name="bk.exe"
    fi

    # Copy binary
    cp "target/$target/release/$bin_name" "$out_dir/"

    # Set archive name
    archive_target="$target"
    if [[ "$target" == "x86_64-pc-windows-gnu" ]]; then
        archive_target="windows"
    fi

    # Compress binary with version and target in filename
    tar czf "dist/bk-${VERSION}-${archive_target}.tar.gz" -C "$out_dir" "$bin_name"

    # If windows, also compress as zip
    if [[ "$target" == *windows* ]]; then
        (cd "$out_dir" && zip "../bk-${VERSION}-${archive_target}.zip" "$bin_name")
    fi
done

for target in "${targets[@]}"; do
  if [ -e "dist/${target}" ]; then
    rm -r "dist/${target}"
  fi
done