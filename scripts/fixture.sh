#!/usr/bin/env sh
# Build a small sample tree with several artifact kinds and print its path, so a
# change can be tried against real files in seconds:
#
#   make fixture              # prints the path
#   disko "$(make -s fixture)"
#
# Idempotent: re-running recreates the same tree. Nothing outside it is touched.
set -eu

root="${TMPDIR:-/tmp}/disco-fixture"
rm -rf "$root"
mk() { mkdir -p "$(dirname "$1")"; head -c "$2" /dev/zero > "$1"; }

mkdir -p "$root/web"; echo '{}' > "$root/web/package.json"
mk "$root/web/node_modules/left-pad/index.js" 3000000
mk "$root/web/src/app.js" 2000

mkdir -p "$root/svc"; echo '[package]' > "$root/svc/Cargo.toml"
mk "$root/svc/target/debug/svc" 5000000
mk "$root/svc/src/main.rs" 500

mkdir -p "$root/ml"; echo 'print(1)' > "$root/ml/train.py"
mkdir -p "$root/ml/.venv"; echo 'home = /usr' > "$root/ml/.venv/pyvenv.cfg"
mk "$root/ml/.venv/lib/torch.so" 8000000
mk "$root/ml/__pycache__/train.pyc" 20000

# Looks like an artifact but is not one: no Cargo.toml beside it.
mk "$root/docs/target/notes.txt" 1000
# An old artifact for --older-than.
mkdir -p "$root/old"; echo '{}' > "$root/old/package.json"
mk "$root/old/node_modules/x.js" 100000
touch -t 202401010000 "$root/old/node_modules/x.js" "$root/old/node_modules" "$root/old"

printf '%s\n' "$root"
