ENTRY(initiate)

MEMORY
{
    bootloader_memory : ORIGIN = 0x7C00, LENGTH = 512
    kernel_memory : ORIGIN = 0x100000, LENGTH = 4M
}

SECTIONS {
    .boot : {
        *(.multiboot_header)
    } > bootloader_memory

    .loader : {
        *(.loader)
    } > bootloader_memory

    .text : {
        *(text*)
    } > kernel_memory

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.rel.eh_frame)
        *(.note.gnu.property)
    }
}