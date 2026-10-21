#!/usr/bin/env python3
# Estimates i486 cycles from callgrind profiles of the bench: each executed instruction weighted by its 486 timing.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: cyc486.py <short.out> <long.out> <frames> [--top N]
#
# The two profiles are the same scene run for two lengths (`valgrind --tool=callgrind --dump-instr=yes --compress-pos=no
# --compress-strings=no`), and `frames` is the difference in frames between them, so the start-up cancels out. The weights are the
# Intel i486 datasheet's clock counts with every access a cache hit and no wait states, and a taken or untaken branch counted
# as 2: an estimate of how the instruction mix weighs on a real 486, which DOSBox does not model (it counts every instruction
# as about one cycle, so x87 work looks far cheaper there than it is on the chip). It reads the disassembly of every object
# in the profiles with objdump.
import collections
import re
import subprocess
import sys

X87 = {
    'fld': 3, 'fild': 14, 'fst': 7, 'fstp': 7, 'fist': 30, 'fistp': 31, 'fadd': 10, 'faddp': 10, 'fsub': 10, 'fsubp': 10,
    'fsubr': 10, 'fsubrp': 10, 'fmul': 16, 'fmulp': 16, 'fdiv': 73, 'fdivp': 73, 'fdivr': 73, 'fdivrp': 73, 'fsqrt': 85,
    'fcom': 4, 'fcomp': 4, 'fcompp': 5, 'fucom': 4, 'fucomp': 4, 'fucompp': 5, 'ftst': 4, 'fxam': 8, 'fnstsw': 3, 'fstsw': 3,
    'fldcw': 4, 'fnstcw': 3, 'fxch': 4, 'fchs': 6, 'fabs': 3, 'fldz': 4, 'fld1': 4, 'frndint': 25, 'fiadd': 22, 'fimul': 23,
    'fidiv': 75, 'ficom': 18, 'fsin': 257, 'fcos': 257, 'fsincos': 292, 'fpatan': 289, 'fyl2x': 311, 'f2xm1': 242,
    'fscale': 30, 'fprem': 84, 'fwait': 1, 'fninit': 17, 'fxtract': 19, 'fldpi': 8, 'fldl2e': 8, 'fldln2': 8,
}
INT = {
    'mov': 1, 'movzx': 3, 'movsx': 3, 'lea': 1, 'add': 1, 'sub': 1, 'and': 1, 'or': 1, 'xor': 1, 'cmp': 1, 'test': 1,
    'inc': 1, 'dec': 1, 'neg': 1, 'not': 1, 'shl': 2, 'shr': 2, 'sar': 2, 'rol': 2, 'ror': 2, 'shld': 3, 'shrd': 3,
    'imul': 15, 'mul': 15, 'idiv': 43, 'div': 40, 'push': 1, 'pop': 4, 'call': 3, 'ret': 5, 'jmp': 3, 'nop': 1, 'cdq': 3,
    'set': 4, 'sahf': 2, 'lahf': 3, 'xchg': 3, 'bswap': 1, 'adc': 1, 'sbb': 1, 'leave': 5, 'bt': 3, 'bsf': 10, 'bsr': 10,
    'movs': 3, 'stos': 4, 'lods': 5, 'cmps': 8, 'scas': 6, 'cld': 2, 'std': 2, 'xlat': 4,
}
RMW = {'add', 'sub', 'and', 'or', 'xor', 'adc', 'sbb', 'inc', 'dec', 'neg', 'not', 'shl', 'shr', 'sar'}
STRING = ('movs', 'stos', 'lods', 'cmps', 'scas')

_dis = {}


def disassembly(obj):
    """Address to instruction text for one object, from objdump."""
    if obj not in _dis:
        table = {}
        try:
            text = subprocess.run(['objdump', '-d', '--no-show-raw-insn', '-M', 'intel', obj], capture_output=True, text=True).stdout
        except OSError:
            text = ''
        for line in text.splitlines():
            m = re.match(r'\s*([0-9a-f]+):\s+(.*)', line)
            if m:
                table[int(m.group(1), 16)] = m.group(2).strip()
        _dis[obj] = table
    return _dis[obj]


def cost(ins):
    """(486 cycles, is x87) for one instruction's text."""
    parts = ins.split()
    if parts and parts[0].startswith('rep'):
        parts = parts[1:]
    if not parts:
        return 1, False
    m, ops = parts[0], ' '.join(parts[1:])
    if m in X87:
        c = X87[m]
        if m in ('fadd', 'fsub', 'fmul', 'fdiv', 'fsubr', 'fdivr') and 'QWORD' in ops:
            c -= 2
        return c, True
    if m.startswith('f'):
        return 10, True
    if m.startswith('j'):
        return (3 if m == 'jmp' else 2), False
    base = m
    if m.startswith('set'):
        base = 'set'
    elif m.startswith(STRING) and m != 'movsx':
        base = next(k for k in STRING if m.startswith(k))
    c = INT.get(base, 2)
    mem = '[' in ops
    if mem and base in RMW:
        c += 2 if '[' in ops.split(',')[0] else 1
    elif mem and base in ('cmp', 'test', 'imul'):
        c += 1
    return c, False


def profile(path):
    """Per function: [instructions, cycles, x87 instructions, x87 cycles]."""
    obj = fn = None
    skip = False
    acc = collections.defaultdict(lambda: [0, 0, 0, 0])
    for line in open(path):
        if line.startswith('ob='):
            obj = line.strip()[3:]
        elif line.startswith('fn='):
            fn = line.strip()[3:]
        elif line.startswith('calls='):
            skip = True  # the next cost line is the call's inclusive cost
        elif line.startswith('0x'):
            if skip:
                skip = False
                continue
            fields = line.split()
            n = int(fields[2])
            ins = disassembly(obj).get(int(fields[0], 16)) if obj else None
            c, f = cost(ins) if ins else (2, False)
            e = acc[fn]
            e[0] += n
            e[1] += c * n
            if f:
                e[2] += n
                e[3] += c * n
        else:
            skip = False
    return acc


def main():
    args = sys.argv[1:]
    top = 0
    if '--top' in args:
        i = args.index('--top')
        top = int(args[i + 1])
        del args[i:i + 2]
    short, long_, frames = profile(args[0]), profile(args[1]), float(args[2])
    per = {}
    for fn, b in long_.items():
        a = short.get(fn, [0, 0, 0, 0])
        per[fn] = [(y - x) / frames for x, y in zip(a, b)]
    tot = [sum(v[i] for v in per.values()) for i in range(4)]
    print(f"a frame: {tot[0]:,.0f} instructions, about {tot[1]:,.0f} 486 cycles; x87 {tot[2]:,.0f} instructions "
          f"({100 * tot[2] / max(tot[0], 1):.0f}%), {tot[3]:,.0f} cycles ({100 * tot[3] / max(tot[1], 1):.0f}%)")
    for fn, v in sorted(per.items(), key=lambda kv: -kv[1][1])[:top]:
        name = re.sub(r'::h[0-9a-f]{16}$', '', fn)[-70:]
        print(f"  {v[1]:>10,.0f} cycles {v[0]:>9,.0f} instr {v[3]:>9,.0f} x87 cycles  {name}")


if __name__ == '__main__':
    main()
