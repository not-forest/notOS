ENTRY(initiate)

MEMORY
{
    bootloader_memory : ORIGIN = 0x7C00, LENGTH = 4K
    kernel_memory : ORIGIN = 0x100000, LENGTH = 4M
}

SECTIONS {
    . = 1M;

    .rodata : ALIGN(4K) {
        /* ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
        *(.rodata .rodata.*)
    } > kernel_memory

    .bootloader : ALIGN(4K) {
        *(.loader)
    } > bootloader_memory

    .text : ALIGN(4K) {
        *(.text .text.*)
    } > kernel_memory

    .data : ALIGN(4K) {
        *(.data .data.*)
    } > kernel_memory

    .bss : ALIGN(4K) {
        *(.bss .bss.*)
    } > kernel_memory

    .got : ALIGN(4K) {
        *(.got)
    } > kernel_memory

    .got.plt : ALIGN(4K) {
        *(.got.plt)
    } > kernel_memory

    .data.rel.ro : ALIGN(4K) {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    } > kernel_memory

    .gcc_except_table : ALIGN(4K) {
        *(.gcc_except_table)
    } > kernel_memory

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.rel.eh_frame)
        *(.note.gnu.property)
    }
}