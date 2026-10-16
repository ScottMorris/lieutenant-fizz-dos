#!/bin/bash
# Copies the parts of the js-dos npm package that the Pages site needs into site/vendor/js-dos, with its licence and where it came from.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/vendor-jsdos.sh [version]      (needs npm and tools/fetch-tools.sh's vendor/ for the GPL-2.0 text)
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:-8.5.3}"
OUT=site/vendor/js-dos
WORK="$(mktemp -d "${TMPDIR:-/tmp}/jsdos.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
(cd "$WORK" && npm pack "js-dos@$VERSION" --silent >/dev/null && tar -xzf "js-dos-$VERSION.tgz")
SRC="$WORK/package/dist"
rm -rf "$OUT"
mkdir -p "$OUT/emulators"
cp "$SRC/js-dos.js" "$SRC/js-dos.css" "$OUT/"
# The DOSBox backend and the zip reader; the DOSBox-X backends and networking are not used.
for f in emulators.js wdosbox.js wdosbox.wasm wlibzip.js wlibzip.wasm; do cp "$SRC/emulators/$f" "$OUT/emulators/$f"; done
cp vendor/dosbox/*/doc/licenses/GPL-2.0.txt "$OUT/LICENSE-GPL-2.0.txt"
SUM="$(cd "$WORK" && sha256sum "js-dos-$VERSION.tgz" | cut -d' ' -f1)"
cat > "$OUT/PROVENANCE.txt" <<TXT
js-dos $VERSION (GPL-2.0), https://js-dos.com, source: https://github.com/caiiiycuk/js-dos

Copied unmodified from the npm package js-dos@$VERSION (tarball sha256 $SUM):
  dist/js-dos.js, dist/js-dos.css, and from dist/emulators: emulators.js, wdosbox.js, wdosbox.wasm,
  wlibzip.js, wlibzip.wasm.
The DOSBox-X backends and the networking files in the package are left out because the page does not use them.

The licence is the GNU General Public License, version 2 (LICENSE-GPL-2.0.txt). The corresponding source is
available from the project's repository above.
TXT
du -sh "$OUT"
