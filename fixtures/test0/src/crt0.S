    .arm
__start:
    mov r1, #123
    rrx r2, r1
    ldr r0, =main
    bx r0
