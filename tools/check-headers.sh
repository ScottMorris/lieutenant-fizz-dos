#!/bin/bash
# Lists the source, workflow and script files that lack the licence and copyright header, and fails if there are any.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Checked: .rs .c .h .ts .js .py .sh and workflow .yml files written for this repository. Not checked: the patched copies of the main
# game's crates (m2/sim, m2/game), the ported sound converters (m4/sound), the generated m2/dos-rt/src/ui_data.rs, the vendored
# js-dos, the archive (attic/), and anything a build writes.
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
while IFS= read -r f; do
  if ! head -12 "$f" | grep -q 'SPDX-License-Identifier'; then
    echo "missing header: $f"
    bad=1
  fi
done < <(git ls-files '*.rs' '*.c' '*.h' '*.ts' '*.js' '*.py' '*.sh' '.github/workflows/*.yml' \
  | grep -v -e '^m2/sim/' -e '^m2/game/' -e '^m4/sound/' -e '^m2/dos-rt/src/ui_data.rs$' -e '^site/vendor/' -e '^attic/')
[ "$bad" = 0 ] && echo "every checked file has the licence header"
exit "$bad"
