ENTRY(_start)
MEMORY {
    rom   (rx)  : ORIGIN = 0x08000000, LENGTH = 32M
    iwram (rwx) : ORIGIN = 0x03000000, LENGTH = 32K - 0x100
}
SECTIONS {
    .crt0 : { KEEP(*(.crt0)) } > rom
    .text : { *(.text .text.*) } > rom
    .rodata : { *(.rodata .rodata.*) . = ALIGN(4); } > rom
    .iwram : ALIGN(4) {
        __iwram_start = .;
        *(.iwram .iwram.*)
        *(.data .data.*)
        . = ALIGN(4);
        __iwram_end = .;
    } > iwram AT > rom
    __iwram_lma = LOADADDR(.iwram);
    .bss (NOLOAD) : {
        __bss_start = .;
        *(.bss .bss.* COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > iwram
    /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) *(.ARM.attributes) *(.comment) }
}
