#!/bin/bash
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed -E 's/version *= *"([^"]+)"/\1/')
VERSION="v${VERSION}"

echo "version: ${VERSION}"

echo -e
echo "checking formatting..."
cargo fmt --all --check

echo -e
echo "checking clippy..."
cargo clippy -- -D warnings

echo -e
echo "checking tests..."
cargo test