/**
  * Function definitions for managing A20 line.
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

.extern _BIOS_MAGIC_ADDR_
.global _check_a20

/**
  * @brief Checks the status of A20 line. When having an address X, we expect X + 1MB
  *        to be different than X, otherwise the A20 is disabled. We compare BIOS's magic
  *        number to make sure we get a stable result.
  * @return AX: 1 if A20 line is enabled. 0 otherwise.
  */
_check_a20:
    pushf
    push %ds
    push %es
    push %di
    push %si

    # Prepare addresses.
    xor %ax, %ax
    notw %ax
    movw %ax, %ds    # DS = 0xffff

    movw $(_BIOS_MAGIC_ADDR_ + 0x10), %si

    cmpw    $0xaa55, %ds:(%si)
    je      .check_a20_disabled       # A20 is off, if we end up reading the same magic.

    movw    $1, %ax
    jmp     .check_a20_exit

.check_a20_disabled:
    xorw    %ax, %ax

.check_a20_exit:
    pop     %si
    pop     %di
    pop     %es
    pop     %ds
    popf
    ret
