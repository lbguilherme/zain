#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Workaround: `zig ar` do Zig 0.16 falha ao criar arquivos .a novos.
# Apontamos para o llvm-ar do Homebrew, que funciona normalmente.
export AR_x86_64_unknown_linux_gnu=/opt/homebrew/opt/llvm/bin/llvm-ar

cargo zigbuild --release -p whatsapp --target x86_64-unknown-linux-gnu
cargo zigbuild --release -p agent --target x86_64-unknown-linux-gnu

mkdir -p dist
cp target/x86_64-unknown-linux-gnu/release/whatsapp dist/
cp target/x86_64-unknown-linux-gnu/release/agent dist/
