ARCH := x86_64
KERNEL := build/kernel-$(ARCH).elf
ISO := build/notOS-$(ARCH).iso
COMPILED_OBJECT := target/$(ARCH)-notOS/debug/libnotOS.a
QEMU_ARGS := -serial mon:stdio


LINKER_SCRIPT := src/arch/$(ARCH)/linker.ld
GRUB_CFG := src/arch/$(ARCH)/grub.cfg

ASSEMBLY_SOURCE_FILES := $(wildcard src/arch/$(ARCH)/*.asm)
ASSEMBLY_OBJECT_FILES := $(patsubst src/arch/$(ARCH)/%.asm, build/arch/$(ARCH)/%.o, $(ASSEMBLY_SOURCE_FILES))

.PHONY: all clean run iso

all: $(KERNEL)

clean:
	@rm -rf build

run: $(ISO)
	@qemu-system-x86_64 -cdrom $(ISO) &

iso: $(ISO)

$(ISO): $(KERNEL) $(GRUB_CFG)
	@mkdir -p build/isofiles/boot/grub
	@cp $(KERNEL) build/isofiles/boot/kernel.bin
	@cp $(GRUB_CFG) build/isofiles/boot/grub
	@grub-mkrescue --verbose -o $(ISO) build/isofiles 2> /dev/null
	@rm -rf build/isofiles

$(KERNEL): build $(ASSEMBLY_OBJECT_FILES) $(LINKER_SCRIPT)
	@ld -n -T $(LINKER_SCRIPT) -o $(KERNEL) $(ASSEMBLY_OBJECT_FILES) $(COMPILED_OBJECT)

build:
	@RUST_TARGET_PATH=$(CURDIR) xargo build

test: $(ISO)
	@qemu-system-x86_64 $(QEMU_ARGS) -cdrom $(ISO) -device isa-debug-exit,iobase=0xf4,iosize=0x04 -display none -no-reboot || true &
	@echo "Running tests. To stop the process call 'make stop'"

stop:
	@kill $$(pgrep -x qemu-system-x86)

# Compile assembly files
build/arch/$(ARCH)/%.o: src/arch/$(ARCH)/%.asm
	@mkdir -p $(dir $@)
	@nasm -felf64 $< -o $@
