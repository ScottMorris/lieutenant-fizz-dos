#!/bin/bash
# Prints the tag of the build image for this tree: a hash of everything the image is built from, so a change to the Dockerfile, the
# Rust version or the tool checksums always gives a new tag.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: .github/images/dos-build/tag.sh

set -euo pipefail
cd "$(dirname "$0")/../../.."
cat .github/images/dos-build/Dockerfile rust-toolchain.toml tools/fetch-tools.sh | sha256sum | cut -c1-12 | sed 's/^/sha-/'
