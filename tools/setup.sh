#!/bin/bash
# Prepares a fresh clone for building: checks the tools it needs, installs the Rust toolchain pieces, downloads the DOS toolchain
# into vendor/, and makes sure the main game is checked out where the exporters and `m0` expect it.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/setup.sh [--no-clone] [--no-fetch]
#   --no-clone  do not clone the main game when it is missing (set FIZZ_GAME to where it is instead)
#   --no-fetch  skip the toolchain download (vendor/ is already there, or cached)
#
# The main game lives at ../../liminal-hq/lieutenant-fizz from this repository unless FIZZ_GAME says otherwise. It is only needed to
# regenerate the exported assets (sprites, sound, interface data) and for the native determinism harness in m0; the committed
# assets build the game without it. A checkout that already exists is left alone.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
CLONE=1; FETCH=1
for arg in "$@"; do
  case "$arg" in
    --no-clone) CLONE=0 ;;
    --no-fetch) FETCH=0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done
GAME="${FIZZ_GAME:-$(cd "$ROOT/../.." 2>/dev/null && pwd)/liminal-hq/lieutenant-fizz}"

step() { printf '\n== %s\n' "$1"; }
missing=()
need() { command -v "$1" >/dev/null 2>&1 || missing+=("$1 ($2)"); }

step "Tools"
need bash "the build scripts"
need python3 "tools/coff2go32.py and the screenshot converter"
need zip "tools/make-dist.sh and tools/make-site.sh"
need curl "tools/fetch-tools.sh"
need sha256sum "tools/fetch-tools.sh"
need tar "tools/fetch-tools.sh"
need git "cloning the main game"
need rustup "the Rust toolchain; https://rustup.rs"
need cargo "the Rust toolchain; https://rustup.rs"
need bun "the asset exporters; https://bun.sh"
need gcc "the native i586 checks (with 32-bit libraries: gcc-multilib on Debian and Ubuntu, lib32-gcc-libs on Arch)"
if [ "${#missing[@]}" -gt 0 ]; then
  printf 'Missing:\n'; printf '  - %s\n' "${missing[@]}"
  echo "Install them (this script does not install system packages) and run it again." >&2
  exit 1
fi
echo "all present"

step "Rust toolchain"
# rust-toolchain.toml pins the version and lists rust-src (for -Zbuild-std), clippy, rustfmt and the i586 target.
rustup show active-toolchain >/dev/null
rustup target add i586-unknown-linux-gnu >/dev/null 2>&1 || true
rustc --version
if echo 'int main(void){return 0;}' | gcc -m32 -x c - -o /dev/null 2>/dev/null; then
  echo "32-bit C libraries: present"
else
  echo "warning: gcc cannot build 32-bit programs; install the 32-bit libraries to run the native i586 checks (m5/bench, m0)" >&2
fi

if [ "$FETCH" = 1 ]; then
  step "DOS toolchain (vendor/)"
  if [ -x vendor/djgpp/djgpp/bin/i586-pc-msdosdjgpp-gcc ] && ls vendor/dosbox/*/dosbox >/dev/null 2>&1 && [ -f vendor/cwsdpmi/bin/CWSDPMI.EXE ]; then
    echo "already in vendor/"
  else
    tools/fetch-tools.sh
  fi
fi

step "JavaScript dependencies"
bun install --frozen-lockfile 2>/dev/null || bun install

step "The main game"
if [ -d "$GAME/.git" ] || [ -f "$GAME/package.json" ]; then
  echo "found at $GAME"
elif [ "$CLONE" = 1 ]; then
  mkdir -p "$(dirname "$GAME")"
  git clone --depth 1 https://github.com/liminal-hq/lieutenant-fizz.git "$GAME"
  (cd "$GAME" && bun install --frozen-lockfile 2>/dev/null || (cd "$GAME" && bun install))
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
  (cd "$GAME" && bun run build:wasm)
else
  echo "not found at $GAME; set FIZZ_GAME, or run without --no-clone to clone it" >&2
fi
if [ -d "$GAME" ] && [ ! -f "$GAME/episodes/episode-1/src/wasm/sim.wasm" ]; then
  echo "note: $GAME has no episodes/episode-1/src/wasm/sim.wasm; run 'bun run build:wasm' there before m0/hash.ts or the exporters" >&2
fi

step "Done"
cat <<MSG
Next:
  m2/build.sh            build the simulation for DOS and check it against the web build
  m4/build.sh --no-run   build the playable m4/c/fizz.exe
  tools/check.sh         everything the CI runs
MSG
