# notOS - OS implementation in Rust.

## Overview

**notOS** is an operating system written in Rust from scratch, focusing on learning all aspects of OS development using only the tools provided by the Rust compiler. Certain parts may be temporarily borrowed or imported for learning purposes. Sometimes temporary imports may appear for some specific amount of time, but they will be replaced for own implementations that, I hope, will be robust for this specific OS.

# Run

To run **notOS**, follow these steps:

1. Ensure you have the **Rust Nightly** version installed.
2. Build the kernel and dependencies:
    ```bash
    make all
    ```
3. To run the OS in debug mode with QEMU and GDB:
    ```bash
    make run
    ```
    This command will start QEMU, load the OS with GDB support, and wait for GDB to connect. It will open GDB in a separate terminal.
4. To create an ISO image for running on virtual machines:
    ```bash
    make iso
    ```
    The ISO image will be generated at `build/notOS-x86_64.iso`.
5. To run the release version of the OS:
    ```bash
    make release
    ```
    This command will build and run the release version of the OS with QEMU and GDB.
6. To run the tests:
    ```bash
    make test
    ```
    This command will build and run the tests in QEMU with GDB.
**Note:** The `make test` command requires Python to extract the test results.

## Project Structure

- **kernel_components**: Main entry point for outer structures and objects.
  - **structures**: Collection of basic data structures.
  - **arch_x86_64**: x86_64 architecture-specific modules.
  - **registers**: Modules for handling various registers.
  - **drivers**: Drivers.
  - **sync**: Synchronization primitives like for specific tasks.
  - **memory**: Memory management modules.
  - **task_virtualization**: Modules for task virtualisation, including the scheduler.
- **build**: Folder that will spawn for output files. 
- **arch**: asm files and linker script.
- **tests**: integrated tests.
- **proc_macros**: small extra crate for custom procedure macros.

## Knowledge Base

This project draws inspiration and knowledge from these sources:
- [Writing an OS in Rust (First Edition)](https://os.phil-opp.com/edition-1/) and [Writing an OS in Rust (Second Edition)](https://os.phil-opp.com/) by Philipp Oppermann.
- [OSDev Wiki](https://wiki.osdev.org/Expanded_Main_Page).
- [The Art of Multiprocessor Programming](https://www.amazon.com/Art-Multiprocessor-Programming-Maurice-Herlihy/dp/0123705916) by Maurice Herlihy, Nir Shavit, Victor Luchangco, Michael Spear.
- [Rustonomicon](https://doc.rust-lang.org/nomicon/).
- [Operating Systems: Three Easy Pieces](http://pages.cs.wisc.edu/~remzi/OSTEP/) by Remzi Arpaci-Dusseau, Andrea Arpaci-Dusseau.
- [MMURTL V1.0](http://www.michaelbasta.com/MMP/Lab/DOC/MMURTL.html) by Richard A. Burgess.
- [Rust Cookbook](https://github.com/rust-lang-nursery/rust-cookbook).
- x86 architecture source information: [www.sandpile.org](www.sandpile.org).
- [x86_64 architecture source](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf).

## Additional Information

- All modules are imported inside the `kernel_components`.
- Macros can be accessed within this crate, and the main components are also accessible from here.
- The library can be used to rewrite the main kernel.
