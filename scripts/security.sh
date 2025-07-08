#!/bin/bash
set -e

cargo deny --all-features check
cargo audit --deny warnings

profile="release"
echo "profile is: $profile"
cargo auditable build --profile $profile
cargo audit bin ./target/$profile/bk


