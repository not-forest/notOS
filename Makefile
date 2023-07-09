ARCH := x86_64
KERNEL := build/kernel-$(ARCH).elf
ISO := build/notOS-$(ARCH).iso
COMPILED_OBJECT := target/$(ARCH)-notOS/debug/libnotOS.a

LINKER_SCRIPT := src/arch/$(ARCH)/linker.ld
GRUB_CFG := src/arch/$(ARCH)/grub.cfg

ASSEMBLY_SOURCE_FILES := $(wildcard src/arch/$(ARCH)/*.asm)
ASSEMBLY_OBJECT_FILES := $(patsubst src/arch/$(ARCH)/%.asm, build/arch/$(ARCH)/%.o, $(ASSEMBLY_SOURCE_FILES))

.PHONY: all clean run iso

all: $(KERNEL)

clean:
	@rm -rf build

run: $(ISO)
	@qemu-system-x86_64 -cdrom $(ISO)

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

# Compile assembly files
build/arch/$(ARCH)/%.o: src/arch/$(ARCH)/%.asm
	@mkdir -p $(dir $@)
	@nasm -felf64 $< -o $@
