ENTRY(_start)
MEMORY {
    ewram (rwx) : ORIGIN = 0x02000000, LENGTH = 256K
    iwram (rwx) : ORIGIN = 0x03000000, LENGTH = 32K - 0x100
}
SECTIONS {
    .crt0 : { KEEP(*(.crt0)) } > ewram
    .text : { *(.text .text.*) } > ewram
    .rodata : { *(.rodata .rodata.*) . = ALIGN(4); } > ewram
    .iwram : ALIGN(4) {
        __iwram_start = .;
        *(.iwram .iwram.*)
        *(.data .data.*)
        . = ALIGN(4);
        __iwram_end = .;
    } > iwram AT > ewram
    __iwram_lma = LOADADDR(.iwram);
    .bss (NOLOAD) : {
        __bss_start = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > iwram
    /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) *(.ARM.attributes) *(.comment) }
}
