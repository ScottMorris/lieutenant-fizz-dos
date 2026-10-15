# fizz-dos

A local side repository for exploring a DOS port of Lieutenant Fizz Episode 1 (Rust, a 1.44 MB floppy, DOSBox). It depends on the main repository by path (`../../liminal-hq/lieutenant-fizz`) and changes nothing in it.

## Milestone 0: does the sim survive 32-bit x87 floating point?

`m0` runs the real Episode 1 sim through the same C-ABI the web shell uses (`game_new`, `enter_level`, `step`, `state_get`): 16 levels, 3000 ticks each, one scripted input stream, and prints a hash of the 26 HUD state slots every 60 ticks (800 lines). `m0/hash.ts` does the same through the web build's `sim.wasm`.

| Build | Lines differing from the web build's wasm |
|---|---|
| x86_64 Linux (SSE2) | 0 of 800 |
| i586 Linux (x87, default 64-bit mantissa) | 466 of 800, first divergence level 0 tick 120 |
| i586 Linux, 53-bit precision control (`fldcw 0x027F`) | 0 of 800 |
| i586, 53-bit, `opt-level = 3` instead of `"s"` | 0 of 800 |

The whole `lf-sim` (35) and `lf-episode-1` (205, including the playtest bot that clears levels) test suites also pass on i586, with or without the 53-bit shim (`tools/fpu53.c`, an `LD_PRELOAD` constructor).

So: x87 with the precision control set to 53 bits reproduces the web build's f64 simulation exactly here; left at the 64-bit default it drifts within two seconds. A DOS start-up must run `fldcw` before the first tick.

Not yet covered: a DJGPP libm (the Linux runs used glibc's), a real 486 or 86Box speed check, and the Fudge Bog fixture replay.

## Milestone 1: Rust in a DOS .EXE

`m1/build.sh` builds a `no_std` Rust library for a custom COFF target (`m1/rust/dos.json`: `i486`, x87 only, `_`-prefixed names), links it with a C `main` using DJGPP, and runs the `.EXE` in headless DOSBox Staging. It prints `Hello from Rust in DOSBox` and `rust_hello returned 3.250000` (an f64 returned on the x87 stack through the C ABI).

Two things the link needed, both in `tools/coff2go32.py`:
- DJGPP's `ld` reads only `coff-go32`, and Rust emits PE-style COFF for a `*-windows-gnu`-like target. The tool adds `-(offset + 4)` to the in-place addend of every `REL32` relocation (go32's convention) and folds the PE sections (`.text$x`, `.rdata`, ...) into go32's `.text`, `.data` and `.bss`. Without the section fold, the code is placed past the loaded image and the program dies silently; without the addend fix, calls land 10 bytes into `puts`.
- `compiler_builtins` objects are dropped from the archive (their weak externals are not understood by coff-go32), so DJGPP's libgcc and libc supply those helpers.

Setup: `tools/fetch-tools.sh` downloads DJGPP (v3.4, gcc 12.2.0), DOSBox Staging 0.83.0 and CWSDPMI into `vendor/` (checksummed, not committed); `rustup component add rust-src --toolchain 1.98.0`; building uses `RUSTC_BOOTSTRAP=1` for `-Zbuild-std` and `-Zjson-target-spec` on the pinned stable toolchain. `tools/dosrun.sh` runs any `.EXE` headless and prints what it wrote to stdout (a crash dump goes to stderr and is not captured).

Not yet: panics and allocation (`alloc` builds but has no allocator), the sim crates built for this target, and a real 486 timing check.

## Starter app

`starter` is a title screen that hands over to the real sim and prints the draw list as text (`fizz-starter --auto --seconds 5 --level 0`, add `--x87-53` on i586). Its `Host` trait is the seam a DOS front end would fill in. Its output on i586 (53-bit) and x86_64 is byte-identical. The text view is crude: the sprite table is a placeholder, so it shows presence only.

## Running it

```
cargo build --release                      # x86_64
cargo build --release --target i586-unknown-linux-gnu
bun m0/hash.ts > out/wasm.txt; ./target/release/m0 > out/x86_64.txt
./target/i586-unknown-linux-gnu/release/m0 --x87-53 > out/i586-53.txt
diff out/wasm.txt out/i586-53.txt && echo identical
```

Needs `rustup target add i586-unknown-linux-gnu` and 32-bit glibc (`lib32-glibc`, `lib32-gcc-libs` on Arch). Tests on x87: `CARGO_TARGET_DIR=$PWD/target-tests cargo test --target i586-unknown-linux-gnu --no-run` in the main repo, then run the binaries with `LD_PRELOAD=$PWD/tools/fpu53.so`.
