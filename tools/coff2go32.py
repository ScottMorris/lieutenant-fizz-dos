#!/usr/bin/env python3
# Makes PE-style COFF objects from rustc linkable by DJGPP's ld: fixes REL32 addends and folds sections into go32 ones.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

"""Make PE-style COFF objects (what rustc emits for a *-windows-gnu target) linkable by DJGPP's ld.

DJGPP's coff-go32 expects the in-place addend of a PC-relative relocation to include -(offset of the field + 4); the
PE convention stores only the real addend. This patches every IMAGE_REL_I386_REL32 relocation and rewrites the
archive without the compiler_builtins objects (DJGPP's libgcc supplies those helpers, and their weak externals are
not understood by coff-go32).

usage: coff2go32.py <in.a> <out.a> <ar tool>
"""
import os, struct, subprocess, sys, tempfile

REL32 = 0x14


def section_name(data: bytearray, base: int, strtab: int) -> str:
    raw = bytes(data[base : base + 8])
    if raw[:1] == b"/":
        off = int(raw[1:].split(b"\0")[0])
        end = data.index(b"\0", strtab + off)
        return data[strtab + off : end].decode()
    return raw.split(b"\0")[0].decode()


# go32 images have three loaded sections; PE objects have many (.text$x, .rdata, .xdata ...). Fold them in.
STYP = {"text": 0x20, "data": 0x40, "bss": 0x80, "info": 0x200}


def classify(name: str) -> tuple[str, int]:
    base = name.split("$")[0]
    if base == ".text":
        return ".text", STYP["text"]
    if base == ".bss":
        return ".bss", STYP["bss"]
    if base in (".data", ".rdata", ".xdata", ".pdata", ".ctors", ".dtors", ".CRT"):
        return ".data", STYP["data"]
    return ".drectve", STYP["info"]


def patch(path: str) -> int:
    data = bytearray(open(path, "rb").read())
    machine, nsects = struct.unpack_from("<HH", data, 0)
    assert machine == 0x14C, f"{path}: not an i386 COFF object"
    symptr, nsyms = struct.unpack_from("<II", data, 8)
    strtab = symptr + nsyms * 18
    count = 0
    for i in range(nsects):
        base = 20 + i * 40
        new_name, flags = classify(section_name(data, base, strtab))
        data[base : base + 8] = new_name.encode().ljust(8, b"\0")
        struct.pack_into("<I", data, base + 36, flags)
        rawptr, relptr = struct.unpack_from("<II", data, base + 20)[0:2]
        # section header: name[8] vsize vaddr rawsize rawptr relocptr lnnoptr nreloc nlnno flags
        rawsize = struct.unpack_from("<I", data, base + 16)[0]
        rawptr = struct.unpack_from("<I", data, base + 20)[0]
        relptr = struct.unpack_from("<I", data, base + 24)[0]
        nreloc = struct.unpack_from("<H", data, base + 32)[0]
        for r in range(nreloc):
            vaddr, _sym, rtype = struct.unpack_from("<IIH", data, relptr + r * 10)
            if rtype == REL32 and rawptr:
                at = rawptr + vaddr
                (addend,) = struct.unpack_from("<i", data, at)
                struct.pack_into("<i", data, at, addend - (vaddr + 4))
                count += 1
    open(path, "wb").write(data)
    return count


def main() -> None:
    src, dst, ar = sys.argv[1:4]
    with tempfile.TemporaryDirectory() as tmp:
        src_abs = os.path.abspath(src)
        subprocess.run(["ar", "x", src_abs], cwd=tmp, check=True)
        objs = sorted(f for f in os.listdir(tmp) if f.endswith(".o"))
        keep, total = [], 0
        for o in objs:
            if o.startswith("compiler_builtins"):
                continue
            total += patch(os.path.join(tmp, o))
            keep.append(o)
        out = os.path.abspath(dst)
        if os.path.exists(out):
            os.remove(out)
        subprocess.run([ar, "rcs", out] + keep, cwd=tmp, check=True)
    print(f"{len(keep)} objects kept, {total} REL32 relocations patched -> {dst}")


main()
