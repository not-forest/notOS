/**
  * Memory layout for bootloader on x86 architecture.
  *
  * Copyright (c) 2026 not-forest
  *
  * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  * SOFTWARE.
  * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
  **/

ENTRY(_start0)

MEMORY
{
    boot0 (rx)  : ORIGIN = 0x7c00, LENGTH = 512     /* First stage bootloader (MBR sector) */
    boot1 (rx)  : ORIGIN = 0x7e00, LENGTH = 600K    /* Second stage bootloader */
}

_PARTITION_TABLE_ADDR_  = ABSOLUTE(ORIGIN(boot0) + 446);
_BIOS_MAGIC_ADDR_       = ABSOLUTE(ORIGIN(boot0) + 510);

SECTIONS
{
    .text : AT(ORIGIN(boot0))
    {
        _BOOT0_STACK_TOP_ = .;
        *boot0.o(.text .text.*)
    } > boot0

    .rodata :
    {
        *boot0.o(.rodata .rodata.*)
    } > boot0

    .partition_table : AT(_PARTITION_TABLE_ADDR_)
    {
        QUAD(0) QUAD(0)
        QUAD(0) QUAD(0)
        QUAD(0) QUAD(0)
        QUAD(0) QUAD(0)
    } > boot0

    .magic : AT(_BIOS_MAGIC_ADDR_)
    {
        SHORT(0xaa55)   /* BIOS magic number. */
    } > boot0

    .text : AT(ORIGIN(boot1)) 
    {
        _SECOND_STAGE_ADDR_ = .;
        *(.text .text.*)
    } > boot1

    .rodata : 
    {
        *(.rodata .rodata.*)
    } > boot1

    .data : 
    {
        *(.data .data.*)
    } > boot1

    /**
      * Calculating how many sectors do we need to copy from the first stage
      * bootloader using BIOS interrupts. For optimization purposes we shall
      * only copy the amount of sectors, which would be enough to cover the
      * entire code within the second stage bootloader. 
      *
      * This also includes code, read-only constants and global variables. 
      **/
    _SECTORS_AMOUNT_ = (. - _SECOND_STAGE_ADDR_ + 511) / 512;

    .bss : 
    {
        *(.bss .bss.*)
    } > boot1
    
    /DISCARD/ : 
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
    }
}
