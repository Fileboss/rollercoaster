#!/bin/sh
# Builds Rollercoaster-<version>-linux-x86_64.tar.gz in target/release/:
# the rollercoaster and looping-cli binaries, README.md and LICENSE. Built
# by the CI on an older Ubuntu, so that its glibc suits most distributions.
#   tools/linux/package.sh
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release -p rollercoaster -p looping-cli

out=target/release
name=Rollercoaster-$version-linux-x86_64
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/$name"
cp "$out/rollercoaster" "$out/looping-cli" README.md LICENSE "$stage/$name/"
tar -czf "$out/$name.tar.gz" -C "$stage" "$name"
echo "$out/$name.tar.gz"
