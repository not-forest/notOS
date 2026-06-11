/**
  * Low level functions to operate on VGA buffer. Used to print panic error
  * messages during first stage bootloader (BOOT0).
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

.code16
.section .text
.global _panic

/**
 * @brief Halts the CPU and prints a terminal error message directly to VGA memory.
 * @param SI: Pointer to a null-terminated string (\0) containing the error message.
 */
_panic:
    movw $0xB800, %ax
    movw %ax, %es
    xorw %di, %di           # Start at row 0, column 0 (top-left)

    movb $0x4F, %ah         # Color formats.

.panic_loop:
    lodsb                   # Load character from DS:SI into AL, increment SI
    testb %al, %al          # Check for null terminator
    jz .panic_hang

    movw %ax, %es:(%di)     # Write character (AL) and attribute (AH) to screen
    addw $2, %di            # Advance 2 bytes to next screen character slot
    jmp .panic_loop

.panic_hang:
    cli
    hlt
    jmp .panic_hang
