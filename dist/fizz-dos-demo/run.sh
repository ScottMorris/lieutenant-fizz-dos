#!/bin/sh
# macOS and Linux: ./run.sh (needs the `dosbox` or `dosbox-staging` command on your PATH).
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

cd "$(dirname "$0")"
if command -v dosbox-staging >/dev/null 2>&1; then exec dosbox-staging --conf DOSBOX.CONF; fi
exec dosbox --conf DOSBOX.CONF
