@ Minimal GBA startup: cartridge header, stacks, .iwram/.data copy, .bss clear.
    .section .crt0, "ax"
    .arm
    .global _start
_start:
    b       start_vector
    .space  188             @ header, filled in by tools/fixrom.py
start_vector:
    mov     r0, #0x12       @ IRQ mode
    msr     cpsr_c, r0
    ldr     sp, =0x03007FA0
    mov     r0, #0x1F       @ System mode, interrupts on
    msr     cpsr_c, r0
    ldr     sp, =0x03007F00

    ldr     r0, =__iwram_lma
    ldr     r1, =__iwram_start
    ldr     r2, =__iwram_end
1:  cmp     r1, r2
    ldrlo   r3, [r0], #4
    strlo   r3, [r1], #4
    blo     1b

    ldr     r1, =__bss_start
    ldr     r2, =__bss_end
    mov     r3, #0
2:  cmp     r1, r2
    strlo   r3, [r1], #4
    blo     2b

    ldr     r0, =main
    mov     lr, pc
    bx      r0
3:  b       3b
    .pool
