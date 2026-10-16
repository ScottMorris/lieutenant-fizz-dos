#!/bin/bash
# Keeps m2/sim and m2/game, the DOS copies of the main game's simulation crates, in step with the main game.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The copies are the main game's crates plus a small patch (m2/patches): `no_std` with `alloc`, the `fmath` traits over `libm`, `core`
# paths, and Cargo manifests that stand alone. This script regenerates the copies from the main game and the patches, or checks that
# they match.
#
# usage: tools/sync-sim.sh [--check | --write-patches]
#   (no option)      copy the main game's crates, apply the patches, and replace m2/sim and m2/game with the result
#   --check          do the same in a scratch folder and fail if m2/sim or m2/game differ from it
#   --write-patches  after an intended edit of m2/sim or m2/game, rewrite the patches from the difference with the main game
#
# The main game is read from FIZZ_GAME (default ../../../liminal-hq/lieutenant-fizz from this repository) and is not changed.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
GAME="${FIZZ_GAME:-$(cd "$ROOT/../.." && pwd)/liminal-hq/lieutenant-fizz}"
MODE="${1:-sync}"
case "$MODE" in sync|--check|--write-patches) ;; *) echo "unknown option: $MODE" >&2; exit 2 ;; esac

# name | the crate in the main game | its copy here
PAIRS=("sim|$GAME/crates/sim|m2/sim" "game|$GAME/episodes/episode-1/game|m2/game")
for pair in "${PAIRS[@]}"; do
  IFS='|' read -r name upstream _ <<<"$pair"
  [ -d "$upstream" ] || { echo "the main game's $name crate is not at $upstream (set FIZZ_GAME; tools/setup.sh clones it)" >&2; exit 1; }
done
WORK="$(mktemp -d "${TMPDIR:-/tmp}/sync-sim.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# A copy of a crate without build output.
copy() { mkdir -p "$2"; (cd "$1" && tar --exclude=target --exclude=Cargo.lock -cf - .) | (cd "$2" && tar -xf -); }

status=0
for pair in "${PAIRS[@]}"; do
  IFS='|' read -r name upstream local <<<"$pair"
  patch_file="m2/patches/$name.patch"
  if [ "$MODE" = "--write-patches" ]; then
    mkdir -p "$WORK/$name" m2/patches
    copy "$upstream" "$WORK/$name/a"
    copy "$local" "$WORK/$name/b"
    (cd "$WORK/$name" && diff -ruN a b > patched.diff) || true
    cp "$WORK/$name/patched.diff" "$patch_file"
    echo "wrote $patch_file ($(grep -c '^[-+][^-+]' "$patch_file") changed lines)"
    continue
  fi
  copy "$upstream" "$WORK/$name"
  patch -s -p1 -d "$WORK/$name" < "$patch_file" || { echo "$patch_file no longer applies to the main game's $name crate; update $local by hand and run --write-patches" >&2; exit 1; }
  if [ "$MODE" = "--check" ]; then
    mkdir -p "$WORK/$name.here"; copy "$local" "$WORK/$name.here"
    if diff -r "$WORK/$name" "$WORK/$name.here" >/dev/null; then
      echo "$local matches the main game plus $patch_file"
    else
      echo "$local differs from the main game plus $patch_file:" >&2
      diff -rq "$WORK/$name" "$WORK/$name.here" >&2 || true
      status=1
    fi
  else
    find "$local" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
    copy "$WORK/$name" "$local"
    echo "updated $local from the main game"
  fi
done
exit $status
