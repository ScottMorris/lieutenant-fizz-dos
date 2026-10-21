# The build image for MS-DOS programs written in Rust and C: DJGPP, CWSDPMI and DOSBox Staging, a pinned Rust with the i586 and
# wasm32 targets, Bun, and the 32-bit and disk-image tools. Workflows run their jobs in it, so a build starts from the same
# tools every time.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Build from the repository's root, with the Rust version from rust-toolchain.toml:
#   docker build -f .github/images/dos-build/Dockerfile --build-arg RUST_VERSION=1.98.0 .
# The tag comes from .github/images/dos-build/tag.sh; README.md in this folder says how to change what is in the image.

FROM ubuntu:24.04@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac

ENV DEBIAN_FRONTEND=noninteractive

# Links the package to the repository that publishes it, so that repository's workflows can pull it.
LABEL org.opencontainers.image.source="https://github.com/ScottMorris/lieutenant-fizz-dos" \
      org.opencontainers.image.description="DJGPP, DOSBox Staging, CWSDPMI, Rust and Bun for building MS-DOS programs"

# gcc-multilib builds the native 32-bit checks; libfl2 is loaded by DJGPP's tools; git is for the scripts that list tracked files; patch is
# for tools/sync-sim.sh.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      bzip2 ca-certificates curl dosfstools gcc gcc-multilib git libfl2 mtools patch python3 unzip xz-utils zip \
 && rm -rf /var/lib/apt/lists/*

# A job runs as root on a checkout that belongs to the runner's user.
RUN git config --system --add safe.directory '*'

# DJGPP, DOSBox Staging and CWSDPMI, checked against the checksums in tools/fetch-tools.sh, which lays them out as vendor/djgpp,
# vendor/dosbox and vendor/cwsdpmi. They go to /opt/dos-toolchain, which tools/setup.sh links as vendor/ when DOS_TOOLCHAIN is set.
COPY tools/fetch-tools.sh /build/tools/fetch-tools.sh
RUN /build/tools/fetch-tools.sh \
 && rm -rf /build/vendor/dl \
 && mv /build/vendor /opt/dos-toolchain \
 && rm -rf /build
ENV DOS_TOOLCHAIN=/opt/dos-toolchain

# Rust, installed where every user can reach it (a container job sets HOME to something else).
ARG RUST_VERSION
RUN test -n "$RUST_VERSION" || { echo "RUST_VERSION is needed" >&2; exit 1; }
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSfL https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init -o /tmp/rustup-init \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --profile minimal --default-toolchain none --no-modify-path \
 && rm /tmp/rustup-init \
 && rustup toolchain install "$RUST_VERSION" --profile minimal \
      -c rustfmt -c clippy -c rust-src -t i586-unknown-linux-gnu -t wasm32-unknown-unknown \
 && rustup default "$RUST_VERSION" \
 && chmod -R a+rwX "$RUSTUP_HOME" "$CARGO_HOME"

COPY --from=oven/bun:1.4.3-debian@sha256:923bedb355f7746666659f71e724998159a5c8c54bd1cb1030e1975a0698415b /usr/local/bin/bun /usr/local/bin/bun
