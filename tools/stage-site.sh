#!/bin/bash
# Stages the GitHub Pages site from a release: its browser bundle becomes site/game.jsdos, the two OPL comparison bundles are made from it,
# and the release's version goes in the page's footer.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/stage-site.sh <tag, such as v0.2.0> <the release's -browser.jsdos>
#
# The Pages workflow runs this on a runner. It changes site/index.html and writes the bundles in site/, so on a working copy it leaves
# changes to discard (git checkout -- site/index.html; the bundles are ignored). tools/make-site.sh is the way to build the page from the
# working tree for local use, and leaves the footer saying "development build".

set -euo pipefail

TAG="${1:?usage: tools/stage-site.sh <tag, such as v0.2.0> <the release browser bundle>}"
BUNDLE="$(realpath -- "${2:?usage: tools/stage-site.sh <tag, such as v0.2.0> <the release browser bundle>}")"
cd "$(dirname "$0")/.."
[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]] || { echo "stage-site.sh: the tag must look like v1.2.3" >&2; exit 1; }
for tool in zip unzip sed; do
  command -v "$tool" >/dev/null || { echo "stage-site.sh: $tool is needed" >&2; exit 1; }
done
unzip -tq "$BUNDLE" >/dev/null || { echo "stage-site.sh: $BUNDLE is not a zip" >&2; exit 1; }
unzip -l "$BUNDLE" | grep -q 'FIZZ.EXE' || { echo "stage-site.sh: $BUNDLE holds no FIZZ.EXE" >&2; exit 1; }

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/stage-site.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
rm -f site/game.jsdos site/game-default.jsdos site/game-compat.jsdos site/game-fast.jsdos
cp "$BUNDLE" site/game.jsdos

# The comparison bundles differ from the usual one only in which OPL emulator DOSBox uses (`site/player.js` loads one with ?opl=).
for opl in default compat; do
  rm -rf "$STAGE/bundle"
  mkdir "$STAGE/bundle"
  (cd "$STAGE/bundle" && unzip -q "$BUNDLE")
  grep -q '^oplemu = fast' "$STAGE/bundle/.jsdos/dosbox.conf" || { echo "stage-site.sh: the bundle's dosbox.conf has no 'oplemu = fast' line" >&2; exit 1; }
  sed -i "s/^oplemu = fast/oplemu = $opl/" "$STAGE/bundle/.jsdos/dosbox.conf"
  (cd "$STAGE/bundle" && zip -q -r -X "$OLDPWD/site/game-$opl.jsdos" .)
done

# The version, as a link to the release's page.
URL="https://github.com/ScottMorris/lieutenant-fizz-dos/releases/tag/$TAG"
sed -i "s|<a id=\"version\" href=\"[^\"]*\">[^<]*</a>|<a id=\"version\" href=\"$URL\">$TAG</a>|" site/index.html
grep -q "<a id=\"version\" href=\"$URL\">$TAG</a>" site/index.html || { echo "stage-site.sh: site/index.html has no version link to fill in" >&2; exit 1; }

echo "staged the page for $TAG"
ls -l site/game.jsdos site/game-default.jsdos site/game-compat.jsdos
