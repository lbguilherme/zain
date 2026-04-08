#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo zigbuild --release -p whatsapp --target x86_64-unknown-linux-gnu
cargo zigbuild --release -p agent --target x86_64-unknown-linux-gnu

mkdir -p dist
cp target/x86_64-unknown-linux-gnu/release/whatsapp dist/
cp target/x86_64-unknown-linux-gnu/release/agent dist/
