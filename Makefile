# The main building area
#
# Prerequisites:
# 	nasm (bootloader part), grub2/grub -tools, xorriso, qemu-system-x86_64, gdb, cargo, rust (nigthly), python
ARCH := x86_64

KERNEL := target/$(ARCH)-notOS/debug/notOS
RELEASE := target/$(ARCH)-notOS/release/notOS
TEST_KERNEL := target/test/latest_tests

ISO := build/notOS-$(ARCH).iso
RELEASE_ISO := build/notOS-$(ARCH)-release.iso
TEST_ISO := build/tests/notOS-$(ARCH).iso

GRUB_CFG := src/arch/$(ARCH)/grub.cfg
GDB_PORT := 1234

ASSEMBLY_SOURCE_FILES := $(wildcard src/arch/$(ARCH)/*.asm)
ASSEMBLY_OBJECT_FILES := $(patsubst src/arch/$(ARCH)/%.asm, build/arch/$(ARCH)/%.o, $(ASSEMBLY_SOURCE_FILES))

AR ?= ar
NASM ?= nasm
GRUB_MKRESCUE ?= grub-mkrescue
QEMU ?= qemu-system-x86_64
GDB ?= gdb
CARGO ?= cargo
PYTHON ?= python3

QEMU_FLAGS := -m 20M -s -S -no-reboot -no-shutdown
QEMU_FLAGS += -audio driver=pa,model=virtio,server=/run/user/1000/pulse/native,id=beeper -machine pcspk-audiodev=beeper

.PHONY: all clean run release test iso

all: $(KERNEL)

stop:
	kill $$(pgrep -x $(QEMU))

clean:
	rm -rf build

# Compile assembly files
build/arch/$(ARCH)/%.o: src/arch/$(ARCH)/%.asm
	mkdir -p $(dir $@)
	$(NASM) -felf64 $< -o $@

# Debugging section
run: $(ISO)
	$(QEMU) -cdrom $(ISO) $(QEMU_FLAGS) & \
	echo "Waiting for QEMU to start..."
	$(GDB) -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(KERNEL)" -ex "layout asm"

iso: $(ISO)

$(ISO): $(KERNEL) build_kernel $(GRUB_CFG)
	mkdir -p build/isofiles/boot/grub
	cp $(KERNEL) build/isofiles/boot/kernel.bin
	cp $(GRUB_CFG) build/isofiles/boot/grub
	$(GRUB_MKRESCUE) --verbose -o $(ISO) build/isofiles
	rm -rf build/isofiles

$(KERNEL): $(ASSEMBLY_OBJECT_FILES)
	$(AR) crus build/libbootloader.a $(ASSEMBLY_OBJECT_FILES)

build_kernel:
	RUST_TARGET_PATH=$(CURDIR) $(CARGO) build $(CARGO_FLAGS)

# Release section
release: $(RELEASE_ISO)
	$(QEMU) -cdrom $(RELEASE_ISO) $(QEMU_FLAGS) & \
	echo "Waiting for QEMU to start..."
	$(GDB) -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(RELEASE)" -ex "layout src"

$(RELEASE): $(ASSEMBLY_OBJECT_FILES)
	$(AR) crus build/libbootloader.a $(ASSEMBLY_OBJECT_FILES)

$(RELEASE_ISO): $(RELEASE) build_release $(GRUB_CFG)
	mkdir -p build/isofiles/boot/grub
	cp $(RELEASE) build/isofiles/boot/kernel.bin
	cp $(GRUB_CFG) build/isofiles/boot/grub
	$(GRUB_MKRESCUE) --verbose -o $(RELEASE_ISO) build/isofiles 2> /dev/null
	rm -rf build/isofiles

build_release:
	RUST_TARGET_PATH=$(CURDIR) $(CARGO) build --release $(CARGO_FLAGS)

# Tests
test: $(TEST_ISO)
	$(QEMU) -cdrom $(TEST_ISO) -m 20M -s -S -no-reboot -no-shutdown & \
	echo "Waiting for QEMU to start..."
	$(GDB) -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(TEST_KERNEL)" -ex "layout src"

$(TEST_ISO): $(KERNEL) test_build $(GRUB_CFG)
	mkdir -p build/tests/isofiles/boot/grub
	cp $(TEST_KERNEL) build/tests/isofiles/boot/kernel.bin
	cp $(GRUB_CFG) build/tests/isofiles/boot/grub
	$(GRUB_MKRESCUE) --verbose -o $(TEST_ISO) build/tests/isofiles 2> /dev/null
	rm -rf build/tests/isofiles

test_build:
	RUST_TARGET_PATH=$(CURDIR) $(CARGO) test --no-run --message-format=json > latest_test.json
	$(PYTHON) extract.py
