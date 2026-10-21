# dos-rust-build

The build image for MS-DOS programs written in Rust and C. The workflows run their jobs in it, so every build starts from the same tools instead of installing them each time.

Published as `ghcr.io/liminal-hq/dos-rust-build:<tag>`, where the tag is `sha-` and twelve hex digits from `tag.sh`.

## What is in it

- Ubuntu 24.04, pinned by digest, with `gcc` and `gcc-multilib` (the native 32-bit checks), `libfl2` (DJGPP's tools load it), `git`, `patch`, `python3`, `zip`, `unzip`, `curl`, `mtools` and `dosfstools` (disk images).
- DJGPP 12.2, DOSBox Staging 0.83 and CWSDPMI in `/opt/dos-toolchain`, checked against the checksums in `tools/fetch-tools.sh` when the image is built. `DOS_TOOLCHAIN` names that folder, and `tools/setup.sh` links it as `vendor/`.
- Rust at the version in `rust-toolchain.toml`, with `rustfmt`, `clippy`, `rust-src` and the `i586-unknown-linux-gnu` and `wasm32-unknown-unknown` targets, in `/usr/local/rustup` and `/usr/local/cargo`.
- Bun, copied from the `oven/bun` image at a pinned version and digest.

Not in it: the repositories, `node_modules`, cargo's registry and `target/` folders, and binaryen (`wasm-opt` would change the game's `sim.wasm`).

## The tag

`tag.sh` hashes the Dockerfile, `rust-toolchain.toml` and `tools/fetch-tools.sh`, so a change to any of them gives a new tag and the image is never stale. `.github/workflows/image.yml` builds and publishes a tag only when it does not exist yet, and `ci.yml` and `release.yml` run their jobs in the tag for the tree they are checking. A pull request that changes the image therefore builds it and runs its checks in it before it merges. A pull request from a fork cannot publish, so its checks fail until a maintainer re-runs them from a branch in this repository.

The workflow also runs monthly and by hand. It publishes a tag only when it is missing, so a published image is never replaced; the monthly run is there to restore one that the registry lost. Fixes to the base image or to Ubuntu's packages come in only when someone updates the digest in the `Dockerfile` (a new tag), not on their own.

## Changing it

- **Another tool version:** edit `tools/fetch-tools.sh` (the URL and the checksum) or the `FROM`, `oven/bun` or apt lines in the `Dockerfile`, and open a pull request. Its checks run in the new image.
- **Another Rust:** edit `rust-toolchain.toml`. The workflow passes its `channel` to the build as `RUST_VERSION`.
- **Rolling back:** revert the commit. The earlier tag is still published; keep package retention from deleting it.

## Building it by hand

From the repository's root:

```bash
docker build -f .github/images/dos-build/Dockerfile --build-arg RUST_VERSION=1.98.0 -t dos-rust-build .
docker run --rm -e NOCOUNT=1 -v "$PWD/../..:/w" -w /w/experiments/fizz-dos dos-rust-build bash -c 'tools/setup.sh --no-clone && tools/check.sh'
```

The repository and the main game must sit side by side in the container as they do on a developer's machine (`experiments/fizz-dos` and `liminal-hq/lieutenant-fizz`). Setting `NOCOUNT=1` skips the benchmark's instruction counter, which a container usually cannot open.
