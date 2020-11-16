# This is the main Makefile for the kernel's compilation
#
# The kernel is divided into two parts:
# - Rust code, which represents most the kernel
# - Assembly and C code
#
# The compilation occurs in the following order:
# - The Makefile compiles Assembly and C code for the required target, creating object files
# - The object files are packed into a library
# - The Rust code is compiled using Cargo
# - Cargo calls the build script, which tells the Rust compiler to link the library previously mentioned
# - Cargo runs the linker with the linker script for the required target
#
# This Makefile also contains several rules used to test the kernel with emulators



# The name of the kernel image
NAME = maestro

# The target architecture. This variable can be set using an environement variable with the same name
KERNEL_ARCH ?= x86
# The target compilation mode. The value an be either `release` or `debug`. Another value may result in an undefined
# behavior. This variable can be set using an environement variable with the same name
KERNEL_MODE ?= debug
# A boolean value telling whether the kernel is compiled for testing or not. This variable can be set using an
# environement variable with the same name
KERNEL_TEST ?= false

# Current directory
PWD := $(shell pwd)

# The path to the architecture specific directory
ARCH_PATH = arch/$(KERNEL_ARCH)/

# The target descriptor file path
TARGET = $(PWD)/$(ARCH_PATH)target.json
# The linker script file path
LINKER = $(PWD)/$(ARCH_PATH)linker.ld

# The C debug flags to use
DEBUG_FLAGS = -D KERNEL_DEBUG -D KERNEL_DEBUG_SANITY -D KERNEL_SELFTEST #-D KERNEL_DEBUG_SPINLOCK

# The C language compiler
CC = i686-elf-gcc
# The C language compiler flags
CFLAGS = -nostdlib -ffreestanding -fstack-protector-strong -fno-pic -mno-red-zone -Wall -Wextra -Werror -lgcc
#ifeq ($(KERNEL_MODE), release)
#CFLAGS += -O3
#else
CFLAGS += -g3 $(DEBUG_FLAGS)
#endif

# Cargo
CARGO = cargo
# Cargo flags
CARGOFLAGS =
ifeq ($(KERNEL_MODE), release)
CARGOFLAGS += --release
endif
ifeq ($(KERNEL_TEST), true)
CARGOFLAGS += --tests
endif

# The Rust language compiler flags
RUSTFLAGS = -Z macro-backtrace

# The name of the library file for the non-Rust code. The code contained in this library is linked using the build script (build.rs)
NON_RUST_LIB_NAME = lib$(NAME).a

# The archive creator program
AR = ar
# The archive creator program flags
ARFLAGS = rc

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
# The list of Rust language source files
RUST_SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")

# The list of directories in the source directory
DIRS := $(shell find $(SRC_DIR) -type d)
# The list of object directories
OBJ_DIRS := $(patsubst $(SRC_DIR)%, $(OBJ_DIR)%, $(DIRS))

# The list of all sources to compile
SRC := $(ASM_SRC) $(C_SRC)

# TODO
CRTI_OBJ = $(OBJ_DIR)crti.s.o
# TODO
CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

# The list of assembly objects
ASM_OBJ := $(patsubst $(SRC_DIR)%.s, $(OBJ_DIR)%.s.o, $(ASM_SRC))
# The list of C language objects
C_OBJ := $(patsubst $(SRC_DIR)%.c, $(OBJ_DIR)%.c.o, $(C_SRC))

# TODO
CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
# TODO
CRTN_OBJ = $(OBJ_DIR)crtn.s.o

# The list of objects
OBJ := $(ASM_OBJ) $(C_OBJ)
# Object files that are result of code which is part of the sources
INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
# TODO
OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

# Flags for the QEMU emulator
QEMU_FLAGS = -cdrom $(NAME).iso -device isa-debug-exit,iobase=0xf4,iosize=0x04

# The rule to compile everything
all: tags $(NAME) iso

# The rule to compile the kernel image
$(NAME): $(NON_RUST_LIB_NAME) $(RUST_SRC) $(LINKER) Makefile
	RUSTFLAGS="$(RUSTFLAGS) --cfg kernel_mode=\"$(KERNEL_MODE)\" -C link-arg=-T$(LINKER)" $(CARGO) build $(CARGOFLAGS) --target $(TARGET) --verbose
	cp $(shell ls -1 target/target/debug/deps/maestro-* | head -n 1) $@
ifeq ($(KERNEL_MODE), release)
	$(STRIP) $(NAME)
endif

# The rule to compile non-Rust code into a separate library for linking
$(NON_RUST_LIB_NAME): $(OBJ_DIRS) $(INTERNAL_OBJ)
	$(AR) $(ARFLAGS) $@ $(INTERNAL_OBJ)

# The rule to create object directories
$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

# The rule to compile assembly objects
$(OBJ_DIR)%.s.o: $(SRC_DIR)%.s $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to compile C language objects
$(OBJ_DIR)%.c.o: $(SRC_DIR)%.c $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# Empty rule used to check if the file has been changed or not
$(TARGET):

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
	qemu-system-i386 $(QEMU_FLAGS) -d int

# The rule to run a CPU test of the kernel using QEMU (aka running the kernel and storing a lot of logs into the
# `cpu_out` file)
cputest: iso
	qemu-system-i386 $(QEMU_FLAGS) -d int,cpu >cpu_out 2>&1

# The rule to test the kernel using Bochs. The configuration for Bochs can be found in the file `.bochsrc`
bochs: iso
	bochs

# The rule to run virtualbox
virtualbox: iso
	virtualbox

.PHONY: all iso clean fclean re test debug bochs
