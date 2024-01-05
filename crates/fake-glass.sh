#!/usr/bin/env bash

set -eu -o pipefail

base=$( dirname $0 )
if [[ -e "$base/glass/Cargo.toml" ]]; then
    echo "real glass dependency exists -- exiting" && exit 1
fi

mkdir -p "$base/glass" "$base/glass/src"
touch "$base/glass/src/lib.rs"

cat <<EOF > "$base/glass/Cargo.toml"
[package]
name = "glass"
version = "0.1.0"
edition = "2021"
EOF
