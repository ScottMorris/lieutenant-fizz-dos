# Rust on DOS

How this repository builds Rust code into an MS-DOS program, written as a guide for doing the same in another project. It covers the toolchain and the Rust side: the custom target, building `core` and `alloc`, converting the objects for DJGPP's linker, the runtime glue, floating point on the x87, calling between C and Rust, testing and packaging. The game's runtime on the machine (memory, interrupts, VGA, sound, the main loop) is in [`DOS_ARCHITECTURE.md`](DOS_ARCHITECTURE.md); the layers and the determinism rules are in [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Contents

- [What you get](#what-you-get)
- [Prerequisites](#prerequisites)
- [The pipeline](#the-pipeline)
- [The target spec](#the-target-spec)
- [Building core and alloc](#building-core-and-alloc)
- [Converting the objects: coff2go32](#converting-the-objects-coff2go32)
- [Linking with DJGPP](#linking-with-djgpp)
- [The runtime glue](#the-runtime-glue)
- [Calling between C and Rust](#calling-between-c-and-rust)
- [Floating point on the x87](#floating-point-on-the-x87)
- [Running and testing](#running-and-testing)
- [Packaging](#packaging)
- [Pitfalls and debugging](#pitfalls-and-debugging)
- [Walkthrough: hello from Rust on DOS](#walkthrough-hello-from-rust-on-dos)

## What you get

- A **32-bit protected-mode DOS `.EXE`** built by DJGPP (GCC 12.2 for `i586-pc-msdosdjgpp`), which runs under a DPMI host. On plain DOS and in DOSBox that host is **CWSDPMI**, a small `CWSDPMI.EXE` kept next to the program.
- **Rust as a static library**: a `#![no_std]` crate with `alloc`, built for a custom i486 target, linked into the DJGPP program. C provides `main`, the C library (`malloc`, `printf`, files) and anything that touches the hardware; Rust provides whatever you put behind `extern "C"` functions.
- **x87 floating point only** (no MMX or SSE), so the code runs on a 486 with an FPU and on anything later.

There is no Rust `std` on this target, no unwinding and no threads. Everything the program needs from the operating system comes through C.

## Prerequisites

The versions this repository pins and checks:

| Piece | Version | Where it comes from |
|---|---|---|
| Rust | 1.98.0 with `rust-src` | `rust-toolchain.toml`; `rust-src` is needed for `-Zbuild-std` |
| DJGPP cross-compiler | build-djgpp v3.4, GCC 12.2.0, Linux x86-64 | `djgpp-linux64-gcc1220.tar.bz2` from `andrewwutw/build-djgpp` releases |
| DOSBox Staging | 0.83.0, Linux x86-64 | `dosbox-staging/dosbox-staging` releases |
| CWSDPMI | r7 (`csdpmi7b.zip`) | `delorie.com/pub/djgpp/current/v2misc/` |
| Python 3 | any recent | runs `tools/coff2go32.py` |

`tools/fetch-tools.sh` downloads the last three into `vendor/`, checks them against SHA-256 sums and unpacks them; `tools/setup.sh` checks for the host tools, adds the Rust pieces and calls it. DJGPP's binaries load `libfl.so.2` (the `libfl2` package on Debian and Ubuntu, `flex` on Arch and Fedora). The native bench described under [Running and testing](#running-and-testing) also needs `gcc` with 32-bit libraries and the `i586-unknown-linux-gnu` Rust target.

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.98.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["i586-unknown-linux-gnu"]
```

The build puts DJGPP's `bin` first on `PATH`, so its tools are found by their prefixed names:

```bash
export PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH"   # i586-pc-msdosdjgpp-gcc, -ar, -nm, -objdump
```

## The pipeline

```
Rust crate (no_std, staticlib)          cargo build --release, target dos.json, -Zbuild-std
        │  libdos_rt.a: PE-style COFF objects
        ▼
tools/coff2go32.py                      fix REL32 addends, fold sections, drop compiler_builtins
        │  dos_rt_go32.a: go32 COFF objects
        ▼
i586-pc-msdosdjgpp-gcc main.c … chkstk.c dos_rt_go32.a -lm
        │
        ▼
FIZZ.EXE  +  CWSDPMI.EXE
```

The two build scripts are short and are the reference. `m4/build.sh`, after the asset steps:

```bash
(cd m2 && RUSTC_BOOTSTRAP=1 cargo build --release -p dos-rt)
tools/coff2go32.py m2/target/dos/release/libdos_rt.a m2/dos_rt_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -fno-toplevel-reorder -o m4/c/fizz.exe m4/c/main.c m4/c/vga.c m4/c/kbd.c m4/c/keymap.c \
  m4/c/chkstk.c m4/c/sound.c m4/c/compat.c m4/c/save.c m2/dos_rt_go32.a -lm
```

`m2/build.sh` does the same with a smaller C program (`m2/c/main.c`) that runs the determinism check.

## The target spec

Rust has no DOS target, so `m2/dos.json` describes one. The trick is to borrow LLVM's **32-bit Windows (MinGW) triple**: it produces i386 COFF objects with `_`-prefixed C symbols, which is close to what DJGPP's linker reads. The objects are not quite right for DJGPP, which is what [coff2go32](#converting-the-objects-coff2go32) fixes.

```json
{
  "llvm-target": "i486-pc-windows-gnu",
  "arch": "x86",
  "os": "windows",
  "vendor": "pc",
  "env": "gnu",
  "binary-format": "coff",
  "cpu": "i486",
  "features": "-mmx,-sse,-sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-avx",
  "data-layout": "e-m:x-p:32:32-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:32-n8:16:32-a:0:32-S32",
  "target-endian": "little",
  "target-pointer-width": 32,
  "target-c-int-width": 32,
  "panic-strategy": "abort",
  "disable-redzone": true,
  "max-atomic-width": 32,
  "frame-pointer": "may-omit",
  "executables": false,
  "is-like-windows": true,
  "dynamic-linking": false,
  "dll-tls-export": false,
  "crt-objects-fallback": "false",
  "linker-flavor": "gnu-cc",
  "linker": "i586-pc-msdosdjgpp-gcc",
  "static-position-independent-executables": false,
  "relocation-model": "static",
  "position-independent-executables": false,
  "stack-probes": {
    "kind": "none"
  }
}
```

Field by field:

| Field | Value | Why |
|---|---|---|
| `llvm-target`, `os`, `env`, `vendor`, `is-like-windows` | `i486-pc-windows-gnu`, `windows`, `gnu`, `pc`, `true` | The MinGW flavour of x86 COFF. LLVM then emits i386 COFF objects that DJGPP's tools can be made to accept. |
| `binary-format` | `coff` | Object files are COFF, not ELF. |
| `cpu` | `i486` | The lowest CPU the code may assume. Code generation avoids later instructions (`cmov`, `cmpxchg8b` and so on). |
| `features` | `-mmx,-sse,…,-avx` | Turns every SIMD extension off, so all floating point goes through the x87 and nothing faults on a 486. |
| `data-layout` | `e-m:x-…-f80:32-…-S32` | The i686 Windows layout. `m:x` is x86 COFF name mangling: global symbols get a leading `_`, which is also DJGPP's C convention, so `printf` in Rust links to DJGPP's `_printf`. `S32` is a 4-byte stack alignment. |
| `target-pointer-width`, `target-c-int-width` | `32`, `32` | ILP32, matching DJGPP's C. |
| `panic-strategy` | `abort` | There is no unwinder. Panics go to your `#[panic_handler]`. |
| `disable-redzone` | `true` | No red zone below the stack pointer. |
| `max-atomic-width` | `32` | The i486 has 32-bit `cmpxchg` but no 64-bit compare-and-swap, so `AtomicU64` is not available. |
| `frame-pointer` | `may-omit` | Frees `ebp` on a register-starved CPU. |
| `executables` | `false` | The target only builds libraries. The executable is linked by DJGPP's GCC, not by `rustc`, so the `linker` and `linker-flavor` fields are never exercised by `crate-type = ["staticlib"]`. |
| `dynamic-linking`, `dll-tls-export`, `crt-objects-fallback` | `false` | No DLLs, no Windows CRT objects. |
| `relocation-model`, `*position-independent-executables` | `static`, `false` | Plain absolute code; DJGPP programs are not position independent. |
| `stack-probes` | `{"kind": "none"}` | No inline stack-probe code. Large frames still call `_alloca`; see [The runtime glue](#stack-probes-and-_alloca). |

Cargo names the output directory after the file stem, so a target called `dos.json` builds into `target/dos/release/`.

## Building core and alloc

No precompiled standard library exists for a custom target, so `core`, `alloc` and `compiler_builtins` are compiled from `rust-src` with `-Zbuild-std`. Custom JSON target files are also behind a flag. Both are set in the DOS crates' own Cargo configuration, `m2/.cargo/config.toml`:

```toml
[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
json-target-spec = true

[build]
target = "dos.json"
```

Unstable flags normally need a nightly toolchain. The repository stays on the pinned stable release and sets `RUSTC_BOOTSTRAP=1` for the DOS build only, which lets stable `cargo` and `rustc` accept `[unstable]` options:

```bash
(cd m2 && RUSTC_BOOTSTRAP=1 cargo build --release -p dos-rt)
```

Because `.cargo/config.toml` sets the default target for everything under `m2/`, the DOS crates live in **their own workspace** (`m2/Cargo.toml`); the root workspace lists `m2` under `exclude`. The release profile, in `m2/Cargo.toml`:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
debug = 0
```

With `lto = true` and one codegen unit, the static library holds **one object for all the Rust code** (plus the `compiler_builtins` object, which the converter drops). `opt-level = "s"` keeps the program small; `docs/PERFORMANCE.md` measured `opt-level = 3` at 1 to 6% faster for a larger binary.

The library crate itself declares `crate-type = ["staticlib"]` (`m2/dos-rt/Cargo.toml`). Ordinary `no_std` crates from crates.io work as dependencies; the simulation uses `libm`.

## Converting the objects: coff2go32

DJGPP's `ld` reads only `coff-go32`, the COFF dialect of DJGPP's own GCC. What `rustc` emits for a `*-windows-gnu` target is PE-style COFF, which DJGPP's binutils will happily open (`i586-pc-msdosdjgpp-objdump` reports it as `coff-go32`) and link without a single error, but the program it produces does not work. Three differences matter, and `tools/coff2go32.py` (about 90 lines of Python with no dependencies) handles all of them:

1. **PC-relative relocation addends.** For an `IMAGE_REL_I386_REL32` relocation (type `0x14`, used for `call` and `jmp` targets), PE stores only the real addend in the instruction's field. go32's convention expects the field to hold the addend minus the field's own offset plus 4. The script patches every one:

   ```python
   if rtype == REL32 and rawptr:
       at = rawptr + vaddr
       (addend,) = struct.unpack_from("<i", data, at)
       struct.pack_into("<i", data, at, addend - (vaddr + 4))
   ```

   Without this, calls land a few bytes past their target (in the first experiments, 10 bytes into `puts`).

2. **Sections.** A go32 image has three loaded sections. PE objects have many: one `.text$name` per function, `.rdata`, `.xdata`, `.pdata` and so on. The script renames each section header and sets go32's flag for it:

   ```python
   def classify(name: str) -> tuple[str, int]:
       base = name.split("$")[0]
       if base == ".text":
           return ".text", STYP["text"]
       if base == ".bss":
           return ".bss", STYP["bss"]
       if base in (".data", ".rdata", ".xdata", ".pdata", ".ctors", ".dtors", ".CRT"):
           return ".data", STYP["data"]
       return ".drectve", STYP["info"]
   ```

   Without the fold, the code is placed past the part of the image that is loaded, and the program dies silently.

3. **`compiler_builtins`.** Its objects use weak externals that coff-go32 does not understand. The script leaves them out of the new archive; DJGPP's libc and libgcc supply the same helpers (`memset`, `memcpy`, 64-bit division and so on), and the Rust object simply references them as undefined symbols.

Usage is `coff2go32.py <in.a> <out.a> <ar tool>`. It extracts the archive with the host `ar`, patches the objects in place in a temporary directory, and writes the result with DJGPP's `ar`:

```
$ tools/coff2go32.py m2/target/dos/release/libdos_rt.a m2/dos_rt_go32.a i586-pc-msdosdjgpp-ar
1 objects kept, 1774 REL32 relocations patched -> m2/dos_rt_go32.a
```

Run it after every `cargo build`; it reads the fresh `.a` and never modifies it.

## Linking with DJGPP

Link the converted archive with your C files using DJGPP's GCC, putting the archive after the C files that call into it:

```bash
i586-pc-msdosdjgpp-gcc -O2 -fno-toplevel-reorder -o fizz.exe main.c … chkstk.c dos_rt_go32.a -lm
```

- **`chkstk.c`** provides `_alloca`, which Rust code calls for large stack frames (see below). Leave it out and the link fails with `undefined reference to '_alloca'`.
- **`-lm`** links DJGPP's maths library for the C side. The Rust side gets its maths from the `libm` crate (see [Floating point](#floating-point-on-the-x87)).
- **`-fno-toplevel-reorder`** is for the C side, not Rust: it keeps C functions in source order, so an interrupt handler's code can be locked in memory as the address range from its first function to an empty marker function after it. [`DOS_ARCHITECTURE.md`](DOS_ARCHITECTURE.md) covers interrupt handlers.
- **Every external symbol the Rust object references must be defined**, even ones the program never calls. LTO leaves one object, and the linker takes all of it. `m2/c/main.c`, the determinism check, has no frame-rate or detail settings, but the Rust shell code refers to them, so it carries stubs:

  ```c
  /* The interface's frame rate setting lives in the full game's main loop; the hash check has none. */
  int rate_mode(void) { return 0; }
  void rate_set(int mode) { (void)mode; }
  int rate_active(void) { return 60; }
  ```

  It also links `m4/c/sound.c` and `m4/c/save.c` for the same reason.

`i586-pc-msdosdjgpp-nm -u dos_rt_go32.a` lists what the Rust side expects C to provide.

## The runtime glue

`m2/dos-rt/src/lib.rs` is the whole of the glue: a `#![no_std]` crate with `extern crate alloc`, a heap, a panic handler and the exported functions.

### Entry from C

There is no Rust entry point. DJGPP's start-up code runs C's `main`, which sets the machine up and calls Rust functions exported with `#[no_mangle] pub extern "C"`. Any one-time Rust set-up, such as the x87 precision control, goes in the first function C calls (`fizz_start` in the game, `fizz_m0` in the determinism check).

### The global allocator

`alloc` needs a `#[global_allocator]`. DJGPP's `malloc` is a good one, declared and used directly:

```rust
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
}

struct Heap;

// DJGPP's malloc returns 8-byte aligned blocks, which covers every type the sim uses (f64 at most).
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        debug_assert!(l.align() <= 8);
        malloc(l.size()).cast()
    }
    unsafe fn dealloc(&self, p: *mut u8, _: Layout) {
        free(p.cast());
    }
    unsafe fn realloc(&self, p: *mut u8, _: Layout, n: usize) -> *mut u8 {
        realloc(p.cast(), n).cast()
    }
}

#[global_allocator]
static HEAP: Heap = Heap;
```

The allocator relies on `malloc`'s 8-byte alignment. A type with a larger alignment (`#[repr(align(16))]`) needs over-allocation and manual alignment instead. Never allocate from an interrupt handler: DJGPP's `malloc` is not reentrant, which is why the repository's rule is no `malloc` in an interrupt handler.

### The panic handler

`no_std` requires a `#[panic_handler]`. The game's prints through C's `printf`, flushes and stops:

```rust
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        printf(b"PANIC\n\0".as_ptr().cast());
        if let Some(l) = info.location() {
            printf(b"  at line %d\n\0".as_ptr().cast(), l.line() as c_int);
        }
        fflush(core::ptr::null_mut());
    }
    loop {}
}
```

It prints only the line, not the formatted message, which keeps `core::fmt` out of the panic path. Once the program has set a graphics mode or hooked interrupts, the text may not be visible and the loop never returns the machine to DOS; a handler that calls back into C to restore the screen and the interrupt vectors before exiting is the more polite choice for a program that owns the hardware. `panic = "abort"` is set in both the target and the profile.

### Stack probes and `_alloca`

`"stack-probes": {"kind": "none"}` removes Rust's own inline probes, but the Windows-style code generation still calls `_alloca` (symbol `__alloca`) with the frame size in `EAX` before any function whose frame is larger than a page (4 KB); a function with an 8 KB local array is enough. DJGPP has no helper under that name, so `m4/c/chkstk.c` supplies the usual one in a top-level `asm` block: touch each 4 KB page on the way down, move `ESP`, and return through the moved return address. Copy that file into any project that uses this target.

## Calling between C and Rust

The boundary is plain `extern "C"` both ways. On this target both compilers use the same i386 cdecl convention: arguments on the stack, integers and pointers returned in `EAX`, `f32` and `f64` returned on the x87 stack. The first experiment returned an `f64` from Rust to C (`rust_hello returned 3.250000`).

**C calling Rust.** Export from Rust:

```rust
/// One tick with the given input bits.
#[no_mangle]
pub extern "C" fn fizz_tick(held: u32) {
    video::advance(held);
}

#[no_mangle]
pub extern "C" fn fizz_paint(frame: *mut u8, count: u32) -> u32 {
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, video::W * video::H) };
    video::paint(frame, count as usize)
}
```

and declare in C (`m4/c/main.c`):

```c
extern void fizz_tick(unsigned held);
extern unsigned fizz_build(float alpha);
extern unsigned fizz_paint(unsigned char *frame, unsigned count);
```

**Rust calling C.** Declare the C functions in an `extern "C"` block and define them in C. The game's sound driver and save files are C, called from the Rust shell (`m2/dos-rt/src/shell.rs`, `m2/dos-rt/src/save.rs`):

```rust
extern "C" {
    fn snd_fx(effect: i32);
    fn snd_set_music(on: i32);
    fn save_write(slot: i32, data: *const u8, length: i32) -> i32;
    fn save_read(slot: i32, data: *mut u8, length: i32) -> i32;
}
```

The repository's conventions for the boundary:

- Only integers, `f32`, pointers and lengths cross it; no structs by value. Struct passing and returning is where the Windows and DJGPP conventions are most likely to differ, and keeping to scalars avoids the question.
- Rust turns a `(pointer, length)` pair into a slice at the boundary (`fizz_paint` above) and works with safe slices inside.
- Strings to C are NUL-terminated byte literals (`b"%u %u\n\0"`); strings from C are read up to the NUL with a bound (`fizz_preview`).
- Large data can live in Rust and be lent to C: `fizz_sound_data` and `fizz_sound_len` hand the C sound driver a pointer into an `include_bytes!` blob.
- Variadic C functions such as `printf` can be declared and called from Rust; pass `c_int`, `u32` or `f64` values, as C's default promotions expect.

## Floating point on the x87

### Precision control: 53 bits

The x87 computes in 80-bit registers with a 64-bit mantissa by default, so `f64` arithmetic on a DOS machine does not round the way IEEE `f64` does on WebAssembly or SSE2. For a simulation that has to match another build bit for bit, that is fatal: with the default precision the game's simulation first differs from the web build at level 0, tick 120 (466 of 800 checkpoints differ). Setting the precision-control field of the x87 control word to 53 bits makes every operation round to an `f64` mantissa, and then all 800 checkpoints match:

```rust
/// x87 precision control to a 53-bit mantissa, so f64 arithmetic matches the web build bit for bit.
fn fpu_53() {
    let cw: u16 = 0x027F;
    unsafe { core::arch::asm!("fldcw word ptr [{0}]", in(reg) &cw, options(nostack)) };
}
```

`0x027F` is all exceptions masked, 53-bit precision, round to nearest. It must run before the first floating-point work that matters, and nothing may change it afterwards. Native 32-bit Linux runs get the same effect from `tools/fpu53.c`, an `LD_PRELOAD` constructor, or `m0`'s `--x87-53` flag.

Precision control does not make the x87 an IEEE `f32` machine. `f32` arithmetic is carried at the wider precision and rounded only when a value is stored, so its results can depend on where the optimiser spills registers: the drawing code's float lighting changed a frame checksum when an unrelated counter was added. Code whose results must be stable either works in `f64` with the 53-bit setting or uses integers.

### Maths functions from `libm`

`core` has no `sin`, `sqrt`, `floor` or `exp` for floats; those are `std` methods. The simulation must compute exactly what the web build computes, so its maths functions come from the pure-Rust `libm` crate on every target, never from DJGPP's C library or the x87's own `fsin` and `fsqrt`. The `no_std` copy of the simulation adds a small module, `m2/sim/src/fmath.rs`, with traits that give the `libm` functions method syntax, so call sites read as they did with `std`:

```rust
/// Method-call syntax for the `libm` functions, so `x.floor()` reads the same with and without `std`.
pub trait FloatMath: Sized {
    fn floor(self) -> Self;
    fn sqrt(self) -> Self;
    fn sin(self) -> Self;
    // ceil, cos, exp, hypot, atan2, powi, fract
}

impl FloatMath for f64 {
    fn floor(self) -> f64 {
        libm::floor(self)
    }
    fn powi(self, n: i32) -> f64 {
        libm::pow(self, f64::from(n))
    }
    fn fract(self) -> f64 {
        self - libm::trunc(self)
    }
    // …
}
```

`FloatMath32` does the same for the two `f32` functions the simulation uses (`sin_cos`, `sqrt`). The web build still uses `std`'s functions; they agree with `libm` over the checked input, but only switching the web build to `libm` too would guarantee it.

### Porting a `std` crate to `no_std`

The simulation is the main game's code, so it is not edited by hand. `m2/patches/sim.patch` and `m2/patches/game.patch` hold the whole difference, and `tools/sync-sim.sh` copies the main game's crates and applies them (`--check` compares, `--write-patches` regenerates the patches after an intended edit). The patches do only this:

- `#![no_std]`, `extern crate alloc`, and `#[cfg(test)] extern crate std;` so the crates' tests still run natively, in each crate root;
- at the top of each module, `use alloc::{boxed::Box, vec, vec::Vec};` and `use lf_sim::fmath::{FloatMath, FloatMath32};` (both `#[allow(unused_imports)]`);
- `std::ptr` and `std::f64::consts` to `core::`;
- stand-alone Cargo manifests with `libm = "0.2"` and `crate-type = ["rlib"]`.

The same pattern carries over to any crate that uses `std` only for collections and float methods. Never reorder floating-point operations in shared code for speed: a different order rounds differently.

### Performance on the x87

From `docs/PERFORMANCE.md`, measured on this code:

- **Float-to-integer conversion is expensive.** Each `f32 as i32` is a control-word swap (Rust's `as` truncates, the x87 rounds), a `fistp` (about 31 clocks on a 486) and a saturation range check: about 400 instructions of float decoding per sprite before the hot paths were changed. Placing sprites from the bits of their `f32` positions with integer shifts removed it with identical results.
- **Divisions and square roots cost tens of clocks** (`fdiv` 73, `fsqrt` 85 on a 486). The lighting took a square root and two divisions per lit cell per light; a 1,024-step falloff table over the squared distance in integers replaced it.
- **Keep per-pixel loops in integers or fixed point.** The sprite blitter steps through source pixels with fixed-point increments.
- **DOSBox hides the x87's cost.** It charges about one cycle per instruction, so float-heavy code looks far cheaper there than on a real chip. `m5/bench/cyc486.py` weights instructions by the i486 datasheet's clock counts to show it.
- **Fixed point does not belong in a simulation that must match another build.** It rounds differently from `f64` from the first tick. Use it between the simulation and the screen.

## Running and testing

### Headless DOSBox

`tools/dosrun.sh <dir> "<command line>"` mounts the directory as `C:`, copies `CWSDPMI.EXE` into it if missing, runs the command with stdout redirected to `OUT.TXT`, exits, and prints `OUT.TXT`:

```bash
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout "${DOS_TIMEOUT:-60}" \
  "$HERE"/vendor/dosbox/*/dosbox --noprimaryconf --nolocalconf --conf "${DOS_CONF:-$HERE/tools/dosbox.conf}" \
  -c "mount c $DIR" -c "c:" -c "$* > OUT.TXT" -c "exit" >/dev/null 2>&1 || echo "dosbox exit $?"
```

`DOS_TIMEOUT` (seconds, default 60) stops a program that hangs, such as one stuck in a panic loop; `DOS_CONF` picks another configuration. `tools/dosbox.conf` runs the dynamic core at `cycles = max` with sound off, which is right for correctness runs. Timing runs use `core = normal` and `cycles = fixed 26800`, the usual figure for a 486DX2-66. Only stdout is captured: a DJGPP crash dump goes to stderr and does not appear.

### A determinism check

`m2/build.sh` is a pattern worth copying for any code that has to agree with another build. The same scripted run (16 levels × 3000 ticks, hashing the state every 60 ticks) exists natively (`m0`), through the web build's wasm (`m0/hash.ts`) and as a DOS program (`fizz_m0`); the script runs the DOS one in DOSBox and diffs its 800 lines against the wasm's:

```bash
DOS_TIMEOUT=900 tools/dosrun.sh m2/c "FIZZ.EXE 3000 1" | tr -d '\r' > out/dos-53.lf
if diff -q out/wasm.txt out/dos-53.lf >/dev/null; then
  echo "PASS: $(wc -l < out/dos-53.lf) checkpoints identical to the web build's wasm"
```

DOS writes `\r\n` line ends, hence the `tr`.

### A native 32-bit bench

Running in DOSBox is slow to iterate on and tells you little about where time goes. `m5/bench` builds **the same Rust source natively** for `i586-unknown-linux-gnu`, a target whose floating point is also x87, and counts instructions per frame with the CPU's counter (`perf_event_open`). It is an ordinary `std` binary that pulls in the DOS runtime's modules by path and stubs the C functions they call:

```rust
#[path = "../../../m2/dos-rt/src/video.rs"]
mod video;

// The C side of the DOS build, which this does not have: sound and save files do nothing here.
#[no_mangle]
pub extern "C" fn snd_fx(_: i32) {}
```

```bash
(cd m5/bench && cargo build --release --target i586-unknown-linux-gnu)
NOCOUNT=1 m5/bench/target/i586-unknown-linux-gnu/release/render-bench 300
```

It prints frame checksums too, which `tools/check.sh` compares with `m5/bench/expected-checksums.txt`, and its stubs make menu behaviour testable without DOS (`PAUSETEST=1`). Where the instruction counter cannot be opened (a virtual machine, a CI runner), `m5/bench/profile.sh` counts under callgrind instead, and `profile.sh --cycles` feeds the profiles to `m5/bench/cyc486.py` for an estimate of 486 cycles. Measure there first, then confirm in DOSBox at a fixed cycle count.

## Packaging

- A DJGPP program needs a DPMI host. Ship **`CWSDPMI.EXE` next to the `.EXE`** (and its `CWSDPMI.DOC`, which carries its licence); DJGPP's start-up stub loads it when no other host is running. CWSDPMI may write a swap file, `CWSDPMI.SWP`, which is worth ignoring in Git.
- **File names are 8.3** on DOS: `FIZZ.EXE`, `FIZZ.CFG`, `SLOT1.SAV`. A longer name fails.
- `tools/make-dist.sh` copies `m4/c/fizz.exe` to `FIZZ.EXE`, adds CWSDPMI and zips the demo folder with its `DOSBOX.CONF`, `RUN.BAT`, `run.sh` and `README.TXT`. The game's `.EXE` is about 0.9 MB, so the whole thing fits a 1.44 MB floppy.

## Pitfalls and debugging

- **A link that succeeds is not a working program.** Linking Rust's unconverted archive gives no error and no warning, and the program then prints nothing at all. If a DJGPP program with Rust in it is silent, check that `coff2go32.py` ran on the archive you linked.
- **`undefined reference to '_alloca'`**: a Rust function has a stack frame over 4 KB. Link `chkstk.c`.
- **Undefined references to C functions the program never calls**: the single LTO object references every `extern "C"` function in the crate. Define them or provide stubs (`m2/c/main.c`).
- **Undefined `memcpy`, `memset` or `__udivdi3`-style helpers** would mean DJGPP's libc or libgcc is missing from the link; link with DJGPP's `gcc` rather than calling `ld` directly so they are added.
- **Results drift from another build after a few seconds**: the precision control was not set, was set too late, or was reset. Set it before the first tick and keep a hash comparison in the check script.
- **Results change when unrelated code changes**: `f32` work on the x87 depends on register spills. Use `f64` with the 53-bit setting, or integers. The root `Cargo.toml` keeps a `fast` profile at `opt-level = 3` to check that the simulation's results do not depend on the optimiser.
- **A panic hangs the program** in the handler's loop. In headless runs `DOS_TIMEOUT` ends it; the `PANIC` line and the source line number are in `OUT.TXT` if the program was writing to stdout.
- **A crash shows nothing** headless: DJGPP's register dump goes to stderr, which `dosrun.sh` discards. Run the program in a DOSBox window to read it, or add `printf`s and `fflush(stdout)` around the suspect call.
- **Inspect the objects** with DJGPP's binutils: `i586-pc-msdosdjgpp-objdump -h` shows the sections before and after conversion, `i586-pc-msdosdjgpp-nm -u` the undefined symbols, `i586-pc-msdosdjgpp-objdump -d` the code.
- **DOSBox speed is not 486 speed**: it counts instructions, not clocks. Use `cycles = fixed 26800` for timings and the 486 estimate for x87-heavy code.

## Walkthrough: hello from Rust on DOS

A complete, minimal project built from the repository's pieces: a `no_std` Rust library with a heap, a panic handler, the 53-bit precision control, a `libm` call, a function with a frame larger than a page, and one call back into C. It needs the toolchain from [Prerequisites](#prerequisites) (with this repository checked out, `tools/fetch-tools.sh` provides it in `vendor/`) and three files copied from the repository: `m2/dos.json`, `tools/coff2go32.py` and `m4/c/chkstk.c`.

```
hello/
  rust-toolchain.toml      copied from the repository
  dos.json                 copied from m2/dos.json
  coff2go32.py             copied from tools/coff2go32.py
  .cargo/config.toml
  Cargo.toml
  src/lib.rs
  c/main.c
  c/chkstk.c               copied from m4/c/chkstk.c
```

`.cargo/config.toml`:

```toml
[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
json-target-spec = true

[build]
target = "dos.json"
```

`Cargo.toml`:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["staticlib"]

[dependencies]
libm = "0.2"

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
debug = 0

[profile.dev]
panic = "abort"
```

`src/lib.rs`:

```rust
#![no_std]

extern crate alloc;

use alloc::alloc::{GlobalAlloc, Layout};
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    // Defined in c/main.c.
    fn host_ticks() -> u32;
}

struct Heap;

// DJGPP's malloc returns 8-byte aligned blocks.
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        malloc(l.size()).cast()
    }
    unsafe fn dealloc(&self, p: *mut u8, _: Layout) {
        free(p.cast());
    }
    unsafe fn realloc(&self, p: *mut u8, _: Layout, n: usize) -> *mut u8 {
        realloc(p.cast(), n).cast()
    }
}

#[global_allocator]
static HEAP: Heap = Heap;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        printf(c"PANIC".as_ptr());
        if let Some(l) = info.location() {
            printf(c" at line %d".as_ptr(), l.line() as c_int);
        }
        printf(c"\n".as_ptr());
        fflush(core::ptr::null_mut());
    }
    loop {}
}

/// Called once from C before anything else: x87 precision control to 53 bits.
#[no_mangle]
pub extern "C" fn rust_init() {
    let cw: u16 = 0x027F;
    unsafe { core::arch::asm!("fldcw word ptr [{0}]", in(reg) &cw, options(nostack)) };
}

#[no_mangle]
pub extern "C" fn rust_hello(n: u32) -> f64 {
    let v: Vec<u32> = (1..=n).collect();
    let sum: u32 = v.iter().sum();
    unsafe {
        printf(c"Hello from Rust on DOS: sum 1..%u = %u, host says %u\n".as_ptr(), n, sum, host_ticks());
    }
    libm::sqrt(f64::from(sum))
}

/// An 8 KB frame, which makes the compiler call `_alloca`.
#[no_mangle]
pub extern "C" fn rust_big_frame(seed: u32) -> u32 {
    let mut buf = [0u8; 8192];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (seed as usize + i) as u8;
    }
    core::hint::black_box(&mut buf);
    buf.iter().map(|&b| u32::from(b)).sum()
}
```

`c/main.c`:

```c
#include <stdio.h>

extern void rust_init(void);
extern double rust_hello(unsigned n);
extern unsigned rust_big_frame(unsigned seed);

unsigned host_ticks(void) { return 42; }

int main(void) {
  rust_init();
  double r = rust_hello(10);
  printf("rust_hello returned %f\n", r);
  printf("rust_big_frame returned %u\n", rust_big_frame(3));
  return 0;
}
```

Build, convert, link and run (from `hello/`, with `FIZZ` set to this repository's checkout for the DOSBox runner):

```bash
export PATH="$FIZZ/vendor/djgpp/djgpp/bin:$PATH"
RUSTC_BOOTSTRAP=1 cargo build --release
python3 coff2go32.py target/dos/release/libhello.a hello_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -o c/hello.exe c/main.c c/chkstk.c hello_go32.a -lm
"$FIZZ/tools/dosrun.sh" c "HELLO.EXE"
```

The output:

```
1 objects kept, 17 REL32 relocations patched -> hello_go32.a
Hello from Rust on DOS: sum 1..10 = 55, host says 42
rust_hello returned 7.416198
rust_big_frame returned 1044480
```

`i586-pc-msdosdjgpp-nm -u hello_go32.a` shows what the Rust object needs from C: `__alloca`, `_fflush`, `_free`, `_host_ticks`, `_malloc`, `_memset`, `_printf` and `_putchar` (LLVM turns a `printf` of a lone newline into `putchar`). `hello.exe` is about 180 KB, most of it DJGPP's C library. To ship it, put `HELLO.EXE` and `CWSDPMI.EXE` in the same directory.

From here, the game adds the hardware layer in C ([`DOS_ARCHITECTURE.md`](DOS_ARCHITECTURE.md)) and grows the Rust side behind more `fizz_*` functions; the build stays these four commands.
