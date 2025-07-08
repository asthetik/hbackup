#!/bin/bash
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed -E 's/version *= *"([^"]+)"/\1/')
VERSION="v${VERSION}"

echo "version: ${VERSION}"

cargo fmt --all --check && cargo clippy -- -D warnings

targets=(
    "i686-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "powerpc64-unknown-linux-gnu"
    "i686-pc-windows-msvc"
    "i686-pc-windows-gnu"
    "x86_64-pc-windows-gnu"
    "x86_64-pc-windows-msvc"
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

profile="release"
echo "profile is: $profile"
for target in "${targets[@]}"; do
    echo "==> Building for $target"
    cross build --profile $profile --target "$target"

    # Prepare output directory
    out_dir="dist/$target"
    mkdir -p "$out_dir"

    # Determine binary name
    bin_name="bk"
    if [[ "$target" == *windows* ]]; then
        bin_name="bk.exe"
    fi

    # Copy binary
    cp "target/$target/$profile/$bin_name" "$out_dir/"

    # Compress binary with version and target in filename
    tar czf "dist/bk-${VERSION}-${target}.tar.gz" -C "$out_dir" "$bin_name"
done

for target in "${targets[@]}"; do
  if [ -e "dist/${target}" ]; then
    rm -r "dist/${target}"
  fi
done