#!/bin/bash
# Downloads the DOS toolchain into vendor/ (not committed): DJGPP cross-compiler, DOSBox Staging and CWSDPMI.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p vendor/dl vendor/djgpp vendor/dosbox vendor/cwsdpmi
cd vendor/dl
[ -f djgpp-linux64-gcc1220.tar.bz2 ] || curl -sSfLO https://github.com/andrewwutw/build-djgpp/releases/download/v3.4/djgpp-linux64-gcc1220.tar.bz2
[ -f dosbox-staging-linux-x86_64-v0.83.0.tar.xz ] || curl -sSfLO https://github.com/dosbox-staging/dosbox-staging/releases/download/v0.83.0/dosbox-staging-linux-x86_64-v0.83.0.tar.xz
[ -f csdpmi7b.zip ] || curl -sSfLO https://www.delorie.com/pub/djgpp/current/v2misc/csdpmi7b.zip
cat > SHA256SUMS <<'SUMS'
8464f17017d6ab1b2bb2df4ed82357b5bf692e6e2b7fee37e315638f3d505f00  djgpp-linux64-gcc1220.tar.bz2
d3a94f7f1c3e68a47ec88d61145506c7904452adb0c9c5928cb8cfe2331d6c5c  dosbox-staging-linux-x86_64-v0.83.0.tar.xz
deacda0488e1cdd7c4a9f32fab45662b34c0ed6b2d7d4d13bc07041b62004a8c  csdpmi7b.zip
SUMS
sha256sum -c SHA256SUMS
tar -xjf djgpp-linux64-gcc1220.tar.bz2 -C ../djgpp
tar -xJf dosbox-staging-linux-x86_64-v0.83.0.tar.xz -C ../dosbox
python3 -I -c "import zipfile;zipfile.ZipFile('csdpmi7b.zip').extractall('../cwsdpmi')"
