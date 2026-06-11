/**
  * First stage bootloader assembly code. It's main job is to load the second
  * stage, which is fully Rust based.
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
.section .text      # MBR Sector section for code.
.global _start0     # Export to linker interface ("bootloader.ld")

# Extern constants from bootloader.ld script.
.extern _BOOT0_STACK_TOP_
.extern _SECOND_STAGE_ADDR_
.extern _SECTORS_AMOUNT_

/**
  * MBR sector starting function.
  */
_start0:
    cld             # Clear direction flag. 
    cli             # Disable interrupts.

    xor %ax, %ax    # AX = 0
    movw %ax, %es   # Clear extra segment 
    movw %ax, %ss   # Clear stack segment

    movw $_BOOT0_STACK_TOP_, %sp    # Initialize bootloader's stack top value. The same
                                    # stack is also used for second stage, since memory
                                    # between 0x0500 to 0x7c00 is totally free for our
                                    # usage, which leaves us with whole 30KB of stack.

    movb %dl, (_BOOT_DRIVE_)        # Saving boot drive number.

    # Enabling the A20 line in the following order:
    # - check if BIOS did not enabled it already;
    # - try to enable via BIOS' interrupts;
    # - check if BIOS enabled it;
    # - try enabling via keyboard controller hack;
    # - test if that worked in time-out loop (keyboard controller latency);
    # - try fast port 0x92 method;
    # - test if that worked in time-out loop (fast port can be slow :3);
    # - panic with error message;
    call _check_a20 
    testw  %ax, %ax
    jnz .load_boot1

.a20_bios:
    movw $0x2401, %ax            # BIOS Function: Enable A20 Gate
    int $0x15
    
    call _check_a20
    testw %ax, %ax
    jnz .load_boot1

.a20_keyboard:
    call .keyboard_wait_command
    movb $0xD1, %al              # Command 0xD1: Write Output Port
    outb %al, $0x64

    call .keyboard_wait_command
    movb $0xDF, %al              # Data 0xDF: Turn A20 line ON
    outb %al, $0x60
    call .keyboard_wait_command

    movw $0xFFFF, %cx            # Set high timeout loop countdown
.keyboard_timeout_loop:
    pushw %cx
    call _check_a20
    popw %cx
    testw %ax, %ax
    jnz .load_boot1
    loop .keyboard_timeout_loop  # Decrement CX and retry until 0

.a20_fast:
    inb $0x92, %al
    orb $0x02, %al               # Bit 1: Fast A20 Init
    andb $0xFE, %al              # Avoid resetting the machine (Bit 0)
    outb %al, $0x92

    movw    $0xFFFF, %cx
.fast_timeout_loop:
    pushw   %cx
    call    _check_a20
    popw    %cx
    testw   %ax, %ax
    jnz     .load_boot1
    loop    .fast_timeout_loop

    # Panic with error message if all above methods failed.
    movw    $.a20_panic_msg, %si
    call    _panic 

.load_boot1:
    # Loading second stage bootloader via BIOS functions.
    movw $_SECOND_STAGE_ADDR_, %bx  # Read from address of second stage bootloader.
    movb $_SECTORS_AMOUNT_, %dh     # Defined at compile-time to fully load the whole boot1.
    movb (_BOOT_DRIVE_), %dl        # Provided by BIOS. Was preserved at the entry point.

    call _load_disk;

.start_boot1:
    call _start;

.keyboard_wait_command:
    inb $0x64, %al
    testb $0x02, %al            # Check Bit 1 (Input Buffer Status)
    jnz .keyboard_wait_command  # Loop if busy (1 = Full, 0 = Empty)
    ret

_BOOT_DRIVE_:
    .byte 0x00

.section .rodata
.a20_panic_msg:
    .string "Failed to enable A20."

.include "A20.s"
.include "vga.s"
.include "disk.s"
