# The name of the kernel image
NAME = maestro

# The target architecture. This variable can be set using an environement variable with the same name
KERNEL_ARCH ?= x86
# The target compilation mode. The value an be either `release` or `debug`. Another value may result in an undefined
# behavior. This variable can be set using an environement variable with the same name
KERNEL_MODE ?= debug

# The target descriptor file path
TARGET = arch/$(KERNEL_ARCH)/target.json
# The linker script file path
LINKER = arch/$(KERNEL_ARCH)/linker.ld

# The C debug flags to use
DEBUG_FLAGS = -D KERNEL_DEBUG -D KERNEL_DEBUG_SANITY -D KERNEL_SELFTEST #-D KERNEL_DEBUG_SPINLOCK

# The C language compiler
CC = i686-elf-gcc
# The C language compiler flags
CFLAGS = -nostdlib -ffreestanding -fstack-protector-strong -mno-red-zone -Wall -Wextra -Werror -lgcc
#ifeq ($(KERNEL_MODE), release)
#CFLAGS += -O3
#else
CFLAGS += -g3 $(DEBUG_FLAGS)
#endif

# The Rust language compiler
RUSTC = rustc
# The Rust language compiler flags
RUSTFLAGS = --emit=obj --target=$(TARGET) -Z macro-backtrace
ifeq ($(KERNEL_MODE), release)
RUSTFLAGS += -O
else
RUSTFLAGS += -g
endif

# The linker program
LD = i686-elf-ld
# The linker program flags
LDFLAGS =
ifeq ($(KERNEL_MODE), release)
#LDFLAGS += --gc-sections
endif

# The strip program
STRIP = strip

# The directory containing sources
SRC_DIR = src/
# The directory containing object files to link
OBJ_DIR = obj/

# The list of assembly source files
ASM_SRC := $(shell find $(SRC_DIR) -type f -name "*.s" -and ! -name "crti.s" -and ! -name "crtn.s")
# The list of C language source files
C_SRC := $(shell find $(SRC_DIR) -type f -name "*.c")
# The list of C language header files
HDR := $(shell find $(SRC_DIR) -type f -name "*.h")
# The main Rust source file
RUST_MAIN = src/kernel.rs
# The list of Rust source files
RUST_SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")

# The list of directories in the source directory
DIRS := $(shell find $(SRC_DIR) -type d)
# The list of object directories
OBJ_DIRS := $(patsubst $(SRC_DIR)%, $(OBJ_DIR)%, $(DIRS))

# The list of all sources to compile
SRC := $(ASM_SRC) $(C_SRC) $(RUST_SRC)

# TODO
CRTI_OBJ = $(OBJ_DIR)crti.s.o
# TODO
CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

# The list of assembly objects
ASM_OBJ := $(patsubst $(SRC_DIR)%.s, $(OBJ_DIR)%.s.o, $(ASM_SRC))
# The list of C language objects
C_OBJ := $(patsubst $(SRC_DIR)%.c, $(OBJ_DIR)%.c.o, $(C_SRC))
# The list of Rust language objects
RUST_MAIN_OBJ := $(patsubst $(SRC_DIR)%.rs, $(OBJ_DIR)%.rs.o, $(RUST_MAIN))

# TODO
CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
# TODO
CRTN_OBJ = $(OBJ_DIR)crtn.s.o

# The path to Rust libcore
LIBCORE = rust/libcore.rlib
# The path to Rust libcompiler_builtins
LIBCOMPILER_BUILTINS = rust/libcompiler_builtins.rlib

# The list of objects
OBJ := $(ASM_OBJ) $(C_OBJ) $(RUST_MAIN_OBJ) $(LIBCORE) $(LIBCOMPILER_BUILTINS)
# TODO
INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
# TODO
OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

# The rule to compile everything
all: tags $(NAME) iso

# The rule to compile the kernel image
$(NAME): $(OBJ_DIRS) $(INTERNAL_OBJ) $(LINKER)
	$(LD) $(LDFLAGS) -T $(LINKER) -o $@ $(OBJ_LINK_LIST)
ifeq ($(KERNEL_MODE), release)
	$(STRIP) $(NAME)
endif

# The rule to create object directories
$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

# The rule to compile assembly objects
$(OBJ_DIR)%.s.o: $(SRC_DIR)%.s $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to compile C language objects
$(OBJ_DIR)%.c.o: $(SRC_DIR)%.c $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to compile Rust language objects
$(RUST_MAIN_OBJ): $(RUST_SRC) $(LIBCORE) Makefile $(TARGET)
	$(RUSTC) $(RUSTFLAGS) -L rust/ -o $@ --extern core=$(LIBCORE) $(RUST_MAIN)

# The rule to compile Rust libcore
$(LIBCORE):
	make all -C rust/

# Alias for $(NAME).iso
iso: $(NAME).iso

# The rule to compile the .iso file image, using grub as a bootloader
$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

# The rule to create the `tags` file
tags: $(SRC) $(HDR)
	ctags $(SRC) $(HDR)

# The rule to clean the workspace
clean:
	rm -rf obj/
	make clean -C rust/
	rm -rf iso/
	rm -f tags

# The rule to clean the workspace, including target binaries
fclean: clean
	rm -f $(NAME)
	rm -rf target/
	rm -f $(NAME).iso

# The rule to recompile everything
re: fclean all

# The rule to test the kernel using QEMU
test: iso
	qemu-system-i386 -cdrom $(NAME).iso -d int

# The rule to run a CPU test of the kernel using QEMU (aka running the kernel and storing a lot of logs into the
# `cpu_out` file)
cputest: iso
	qemu-system-i386 -cdrom $(NAME).iso -d int,cpu >cpu_out 2>&1

# The rule to test the kernel using Bochs. The configuration for Bochs can be found in the file `.bochsrc`
bochs: iso
	bochs

# The rule to run virtualbox
virtualbox: iso
	virtualbox

.PHONY: all iso clean fclean re test debug bochs
