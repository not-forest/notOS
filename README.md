# notOS - A Rust OS Implementation

## Overview

**notOS** is a monolithic operating system written in Rust from scratch with no external libraries, focusing on implementing all aspects of OS using only the tools provided by the Rust compiler. The system currently supports kernel-space operations, memory management, process management, hardware interaction, and task scheduling.

## Key Features

- Written purely in Rust, with assembly post-bootloading initialization;
- Implements OS concepts like memory allocation, process management, task switching, interrupts and more;
- Memory management with configurable memory allocators and paging memory model;
- Configurable scheduler structure for multitasking;
- Custom IPC mechanisms and synchronization primitives. Concurrent lock-free structures;
- Hardware abstraction over x86 architecture: controllers, interrupt handling, ports I/O, registers;
- Loading from GRUB with tags parsing;
- Custom dynamic driver management (in progress...);
- ACPI support and power management (in progress...);

### Prerequisites

- **Rust Nightly** version.
- `make`, `qemu`, and `gdb`.

### Building and Running the OS

1. Build the kernel and dependencies:
    ```bash
    make all
    ```

2. To run the OS in debug mode with QEMU and GDB:
    ```bash
    make run
    ```
    This will start QEMU, load the OS with GDB support, and wait for GDB to connect. It will open GDB in a separate terminal.

3. To create an ISO image for running on virtual machines:
    ```bash
    make iso
    ```
    The ISO image will be generated at `build/notOS-x86_64.iso`.

4. To run the release version of the OS:
    ```bash
    make release
    ```
    This will build and run the release version of the OS with QEMU and GDB.

5. To run the tests:
    ```bash
    make test
    ```
    This will build and run the tests in QEMU with GDB.

**Note:** The `make test` command requires Python to extract the test results.

### Project Structure

- **kernel_components**: Main entry point for outer structures and objects.
  - **structures**: Collection of basic data structures.
  - **arch_x86_64**: x86_64 architecture-specific modules.
  - **registers**: Modules for handling various registers.
  - **drivers**: Driver implementations.
  - **sync**: Synchronization primitives.
  - **memory**: Memory management modules.
  - **task_virtualization**: Modules for task virtualization, including the scheduler.
- **build**: Folder for output files.
- **arch**: Assembly files and linker script.
- **tests**: Integrated tests.
- **proc_macros**: Extra crate for custom procedural macros.

### Knowledge Base

This project draws inspiration from various resources:

- [Writing an OS in Rust (First Edition)](https://os.phil-opp.com/edition-1/) and [Second Edition](https://os.phil-opp.com/) by Philipp Oppermann.
- [OSDev Wiki](https://wiki.osdev.org/Expanded_Main_Page)
- [The Art of Multiprocessor Programming](https://www.amazon.com/Art-Multiprocessor-Programming-Maurice-Herlihy/dp/0123705916) by Maurice Herlihy et al.
- [Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [Operating Systems: Three Easy Pieces](http://pages.cs.wisc.edu/~remzi/OSTEP/) by Remzi Arpaci-Dusseau et al.
- [MMURTL V1.0](http://www.michaelbasta.com/MMP/Lab/DOC/MMURTL.html) by Richard A. Burgess.
- [Rust Cookbook](https://github.com/rust-lang-nursery/rust-cookbook)
- [x86 architecture source information](https://www.sandpile.org/)
- [Intel x86_64 Architecture Manual](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf)

### Project Evolution

The project is a constant work in progress. Temporary code and workarounds may exist but are refined over time.

## Additional Information

- All modules are imported inside the `kernel_components`.
- Macros can be accessed within this crate, and the main components are also accessible from there.
- The library can be used to rewrite the main kernel.
