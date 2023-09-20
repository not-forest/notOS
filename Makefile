# The main building area
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


.PHONY: all clean run release test iso

all: $(KERNEL)

stop:
	@kill $$(pgrep -x qemu-system-x86)

clean:
	@rm -rf build

# Compile assembly files
build/arch/$(ARCH)/%.o: src/arch/$(ARCH)/%.asm
	@mkdir -p $(dir $@)
	@nasm -felf64 $< -o $@

# Debugging section
run: $(ISO)
	@qemu-system-x86_64 -cdrom $(ISO) -m 20M -s -S & 
	@echo "Waiting for QEMU to start..."
	@sleep 2
	@gdb -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(KERNEL)" -ex "layout asm"

iso: $(ISO)

$(ISO): $(KERNEL) build_kernel $(GRUB_CFG)
	@mkdir -p build/isofiles/boot/grub
	@cp $(KERNEL) build/isofiles/boot/kernel.bin
	@cp $(GRUB_CFG) build/isofiles/boot/grub
	@grub-mkrescue --verbose -o $(ISO) build/isofiles 2> /dev/null
	@rm -rf build/isofiles

$(KERNEL): $(ASSEMBLY_OBJECT_FILES)
	@ar crus build/libbootloader.a $(ASSEMBLY_OBJECT_FILES)


build_kernel:
	@RUST_TARGET_PATH=$(CURDIR) xargo build


# Release section
release: $(RELEASE_ISO)
	@qemu-system-x86_64 -cdrom $(RELEASE_ISO) -m 10M -s -S & 
	@echo "Waiting for QEMU to start..."
	@sleep 2
	@gdb -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(RELEASE)" -ex "layout asm"

$(RELEASE): $(ASSEMBLY_OBJECT_FILES)
	@ar crus build/libbootloader.a $(ASSEMBLY_OBJECT_FILES)

$(RELEASE_ISO): $(RELEASE) build_release $(GRUB_CFG)
	@mkdir -p build/isofiles/boot/grub
	@cp $(RELEASE) build/isofiles/boot/kernel.bin
	@cp $(GRUB_CFG) build/isofiles/boot/grub
	@grub-mkrescue --verbose -o $(RELEASE_ISO) build/isofiles 2> /dev/null
	@rm -rf build/isofiles

build_release:
	@RUST_TARGET_PATH=$(CURDIR) xargo build --release


#Tests
test: $(TEST_ISO)
	@qemu-system-x86_64 -cdrom $(TEST_ISO) -m 20M -s -S -no-reboot -no-shutdown & 
	@echo "Waiting for QEMU to start..."
	@sleep 2
	@gdb -ex "target remote :$(GDB_PORT)" -ex "symbol-file $(TEST_KERNEL)" -ex "layout asm"

$(TEST_ISO): $(KERNEL) test_build $(GRUB_CFG)
	@mkdir -p build/tests/isofiles/boot/grub
	@cp $(TEST_KERNEL) build/tests/isofiles/boot/kernel.bin
	@cp $(GRUB_CFG) build/tests/isofiles/boot/grub
	@grub-mkrescue --verbose -o $(TEST_ISO) build/tests/isofiles 2> /dev/null
	@rm -rf build/tests/isofiles

test_build:
	@RUST_TARGET_PATH=$(CURDIR) cargo test --no-run --message-format=json > latest_test.json
	@python3 extract.py

