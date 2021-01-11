# This is the main makefile for the kernel's compilation
#
# The kernel is divided into two parts:
# - Rust code, which represents most the kernel
# - Assembly and C code
#
# The compilation occurs in the following order:
# - The makefile calls lib.makefile to compile assembly and C code and pack them into static libraries
# - The Rust code is compiled using Cargo
# - Cargo calls the build script, which tells the Rust compiler to link the libraries previously mentioned
# - Cargo runs the linker with the linker script for the required target
#
# This makefile also contains several rules used to test the kernel with emulators



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
# If true, compiles libraries for userspace testing purpose
USERSPACE_TEST ?= false

# Forcing the KERNEL_TEST option to `false` if building in release mode
ifeq ($(KERNEL_MODE), release)
KERNEL_TEST = false
endif

# The C language compiler
CC = i686-elf-gcc
ifeq ($(USERSPACE_TEST), true)
CC = cc
endif

# Current directory
PWD := $(shell pwd)

# The path to the architecture specific directory
ARCH_PATH = $(PWD)/arch/$(KERNEL_ARCH)/

# The target descriptor file path
TARGET = $(ARCH_PATH)target.json
# The linker script file path
LINKER = $(ARCH_PATH)linker.ld

# Cargo
CARGO = cargo
# Cargo flags
CARGOFLAGS = --verbose
ifeq ($(KERNEL_MODE), release)
CARGOFLAGS += --release
endif
ifeq ($(KERNEL_TEST), true)
CARGOFLAGS += --tests
endif

# The Rust language compiler flags
RUSTFLAGS = -Z macro-backtrace -C link-arg=-T$(LINKER) --cfg kernel_mode=\"$(KERNEL_MODE)\"

# The strip program
STRIP = strip

# The list of Rust language source files
RUST_SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")

# Flags for the QEMU emulator
QEMU_FLAGS = -cdrom $(NAME).iso -device isa-debug-exit,iobase=0xf4,iosize=0x04

# The list of library files
LIB_FILES := lib$(NAME).a mem_alloc/libmem_alloc.a util/libutil.a

# The rule to compile everything
all: $(NAME) iso

# TODO: Fix the incorrect binary in target. This is probably due to the usage of the flag to compile for testing
# The rule to compile the kernel image
$(NAME): $(LIB_FILES) $(RUST_SRC) $(LINKER) Makefile
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) build $(CARGOFLAGS) --target $(TARGET)
	cp `ls -1 target/target/debug/deps/maestro-* | head -n 1` $@
ifeq ($(KERNEL_MODE), release)
	$(STRIP) $(NAME)
endif

# TODO Select the C/assembly compiler according to architecture

# The rule to compile the main kernel library
lib$(NAME).a:
	 CC='$(CC)' make -f lib.makefile

# The rule to compile the memory allocation kernel library
mem_alloc/libmem_alloc.a:
	LIB_NAME='mem_alloc' BUILD_ROOT='mem_alloc' CC='$(CC)' make -f lib.makefile

# The rule to compile the utility kernel library
util/libutil.a:
	LIB_NAME='util' BUILD_ROOT='util' CC='$(CC)' make -f lib.makefile

# Alias for $(NAME).iso
iso: $(NAME).iso

# The rule to compile the .iso file image, using grub as a bootloader
$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

# The rule to clean the workspace
clean:
	make clean -f lib.makefile
	LIB_NAME='mem_alloc' BUILD_ROOT='mem_alloc' make clean -f lib.makefile
	LIB_NAME='util' BUILD_ROOT='util' make clean -f lib.makefile
	rm -rf iso/

# The rule to clean the workspace, including target binaries
fclean: clean
	make fclean -f lib.makefile
	LIB_NAME='mem_alloc' BUILD_ROOT='mem_alloc' make fclean -f lib.makefile
	LIB_NAME='util' BUILD_ROOT='util' make fclean -f lib.makefile
	rm -rf target/
	rm -f $(NAME)
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
