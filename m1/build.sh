#!/bin/bash
# Milestone 1: a Rust (no_std, x87) library linked into a DOS .EXE by DJGPP and run in headless DOSBox.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH"
(cd m1/rust && RUSTC_BOOTSTRAP=1 cargo build --release)
tools/coff2go32.py m1/rust/target/dos/release/libhello_rs.a m1/rust/hello_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -o m1/c/fizz.exe m1/c/main.c m1/rust/hello_go32.a
tools/dosrun.sh m1/c FIZZ.EXE
