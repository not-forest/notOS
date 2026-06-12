/**
  * Memory layout for kernel on x86 architecture.
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

ENTRY(_start)

SECTIONS
{
    /** 
      * Ensure that the multiboot header is at the beginning to support loading from GRUB 
      **/
    .multiboot : AT(ORIGIN(kernel)) 
    {
        KEEP(*(.multiboot_header))
    } > kernel

    .text : 
    {
        *(.text .text.*)
    } > kernel

    .rodata : 
    {
        *(.rodata .rodata.*)
    } > kernel

    .data : 
    {
        *(.data .data.*)
    } > kernel

    .bss : 
    {
        *(.bss .bss.*)
    }

    /DISCARD/ : 
    {
        *(.comment)
        *(.eh_frame)
        *(.rel.eh_frame)
        *(.note.gnu.property)
    }
}
