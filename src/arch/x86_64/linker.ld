ENTRY(initiate)

SECTIONS {
    . = 1M; /* Memory allocation for the bootloader and kernel. */

    .boot :
    {
        *(.multiboot_header)
    }

    .text :
    {
        *(.text)
    }
}