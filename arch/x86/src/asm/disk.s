/**
  * Disk operations using BIOS functions. Used for copying second stage of
  * the bootloader from the first MBR sector.
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

.section .boot
.code16

.global _load_disk

.include "vga.s"

/**
 * @brief Interface to load data from the disk via BIOS Int 13h.
 * @input BX: Buffer offset to load data into (ES must point to the target segment).
 * @input DL: Drive number (passed automatically by the BIOS on boot).
 * @input DH: Number of sectors to read.
 */
_load_disk:
    pusha       
    pushw %dx           # Save the requested sector count (DH) and drive (DL)

    movb $0x02, %ah     # BIOS function 0x02: Read Sectors From Drive
    movb %dh, %al       # AL = Number of sectors to read

    movb $0x00, %ch     # CH = Cylinder 0
    movb $0x02, %cl     # CL = Sector 2 (Sector 1 is your boot0 MBR)
    movb $0x00, %dh     # DH = Head 0

    int $0x13           # BIOS interrupt for disk operations
    jc _disk_error      # Carry Flag is set if a hardware error occurred

    popw %dx
    cmpb %dh, %al       # Compare sectors actually read (AL) with requested (DH)
    jne _sectors_error
    
    popa
    ret

_disk_error:
    movw    $.disk_panic_msg, %si
    call    _panic 

_sectors_error:
    movw    $.sectors_panic_msg, %si
    call    _panic 

.section .boot.rodata
.disk_panic_msg:
    .string "Hardware disk error occured."
.sectors_panic_msg:
    .string "Invalid value of sectors read."
