#!/bin/bash
set -e

cargo deny --all-features check
cargo audit --deny warnings

profiles=(
    "release"
    "release-lto"
)

for profile in "${profiles[@]}"; do
    echo "profile = $profile"
    cargo auditable build --profile $profile
    cargo audit bin ./target/$profile/bk
done


