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

INCLUDE mem.ld

_PARTITION_TABLE_ADDR_  = ABSOLUTE(ORIGIN(boot0) + 446)
_BIOS_MAGIC_ADDR_       = ABSOLUTE(ORIGIN(boot0) + 510)

SECTIONS
{
    . = ORIGIN(boot0);
    _BOOT0_STACK_TOP_ = .;

    .boot :
    {
        *(.boot .boot.*)
    }

    .partition_table : AT(_PARTITION_TABLE_ADDR_)
    {
        /* partition table entry 0 */
        QUAD(0)
        QUAD(0)
        /* partition table entry 1 */
        QUAD(0)
        QUAD(0)
        /* partition table entry 2 */
        QUAD(0)
        QUAD(0)
        /* partition table entry 3 */
        QUAD(0)
        QUAD(0)
    }

    .magic : AT(_BIOS_MAGIC_ADDR_)
    {
        SHORT(0xaa55)   /* BIOS magic number. */
    }

    . = ORIGIN(boot1);
    _SECOND_STAGE_ADDR_ = .;

    .text : {
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    .data : {
        *(.data .data.*)
        /**
          * Calculating how many sectors do we need to copy from the first stage
          * bootloader using BIOS interrupts. For optimization purposes we shall
          * only copy the amount of sectors, which would be enough to cover the
          * entire code within the second stage bootloader. 
          *
          * This also includes code, read-only constants and global variables. 
          **/
        _SECTORS_AMOUNT_ = ABSOLUTE((. + 511) / 512 - ORIGIN(boot1))
    }
    .bss : {
        *(.bss .bss.*)
    }
    .eh_frame : {
        *(.eh_frame .eh_frame.*)
    }
    .eh_frame_hdr : {
        *(.eh_frame_hdr .eh_frame_hdr.*)
    }
}
