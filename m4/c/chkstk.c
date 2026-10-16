/* The `_alloca` helper that Rust's COFF objects call before a function with a large stack frame.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

/* Rust's x86 Windows-style target (the one we borrow for its COFF objects) calls `_alloca` before a function that
   needs a big stack frame, with the size in EAX; it expects the stack pointer moved down by that much. DJGPP has no
   such helper under that name, so this is the usual one: touch each page on the way down, then adjust ESP and
   return through the moved return address. */
__asm__(
    ".text\n"
    ".globl __alloca\n"
    "__alloca:\n"
    "  pushl %ecx\n"
    "  leal 8(%esp), %ecx\n"
    "  cmpl $0x1000, %eax\n"
    "  jb 2f\n"
    "1:\n"
    "  subl $0x1000, %ecx\n"
    "  orl $0, (%ecx)\n"
    "  subl $0x1000, %eax\n"
    "  cmpl $0x1000, %eax\n"
    "  ja 1b\n"
    "2:\n"
    "  subl %eax, %ecx\n"
    "  orl $0, (%ecx)\n"
    "  movl %esp, %eax\n"
    "  movl %ecx, %esp\n"
    "  movl (%eax), %ecx\n"
    "  movl 4(%eax), %eax\n"
    "  pushl %eax\n"
    "  ret\n");
