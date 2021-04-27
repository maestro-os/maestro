# This is the main makefile for the kernel's compilation
#
# The kernel is divided into two parts:
# - Rust code, which represents most the kernel
# - Assembly and C code
#
# The compilation occurs in the following order:
# - The makefile compiles assembly and C code and pack them into static a library
# - The Rust code is compiled using Cargo
# - Cargo calls the build script, which tells the Rust compiler to link the library previously
# mentioned
# - Cargo runs the linker with the linker script for the required target
#
# This makefile also contains several rules used to test the kernel with emulators



# The name of the kernel image
NAME = maestro



# Current directory
PWD := $(shell pwd)



# The path to the script that generates configuration as compiler arguments
CONFIG_ARGUMENTS_SCRIPT = scripts/config_args.sh
# The path to the script that extracts specific configuration attributes
CONFIG_ATTR_SCRIPT = scripts/config_attr.sh

# The path to the configuration file created by the configuration utility
CONFIG_FILE = .config
# Tells whether the configuration file exists
CONFIG_EXISTS = $(shell stat $(CONFIG_FILE) >/dev/null 2>&1; echo $$?)

ifeq ($(CONFIG_EXISTS), 0)
# Configuration as arguments for the compiler
CONFIG_ARGS := $(shell $(CONFIG_ARGUMENTS_SCRIPT))

# The target architecture
CONFIG_ARCH := $(shell $(CONFIG_ATTR_SCRIPT) general_arch)
# The target architecture
CONFIG_DEBUG := $(shell $(CONFIG_ATTR_SCRIPT) debug_debug)
endif



# The path to the architecture specific directory
ARCH_PATH = $(PWD)/arch/$(CONFIG_ARCH)/

# The target descriptor file path
TARGET = $(ARCH_PATH)target.json
# The linker script file path
LINKER = $(ARCH_PATH)linker.ld

# The directory containing sources
SRC_DIR = $(PWD)/src/
# The directory containing object files to link
OBJ_DIR = $(PWD)/obj/

# The name of the library containg the C/Assembly code.
LIB_NAME = lib$(NAME).a

# The C language compiler
CC = i686-elf-gcc # TODO Set according to architecture

# The debug flags for the C compiler
DEBUG_FLAGS = -D KERNEL_DEBUG -D KERNEL_DEBUG_SANITY -D KERNEL_SELFTEST #-D KERNEL_DEBUG_SPINLOCK

# The C language compiler flags
CFLAGS = -nostdlib -ffreestanding -fno-stack-protector -fno-pic -mno-red-zone -Wall -Wextra\
-Werror -lgcc
ifeq ($(CONFIG_DEBUG), false)
CFLAGS += -O3
else
CFLAGS += -g3 $(DEBUG_FLAGS)
endif

# The archive creator program
AR = ar
# The archive creator program flags
ARFLAGS = rc

# The list of assembly source files
ASM_SRC := $(shell find $(SRC_DIR) -type f -name "*.s" -and ! -name "crti.s" -and ! -name "crtn.s")
# The list of C language source files
C_SRC := $(shell find $(SRC_DIR) -type f -name "*.c")
# The list of C language header files
HDR := $(shell find $(SRC_DIR) -type f -name "*.h")

# The list of directories in the source directory
DIRS := $(shell find $(SRC_DIR) -type d)
# The list of object directories
OBJ_DIRS := $(patsubst $(SRC_DIR)%, $(OBJ_DIR)%, $(DIRS))

# The list of all sources to compile
SRC := $(ASM_SRC) $(C_SRC)

# TODO
#CRTI_OBJ = $(OBJ_DIR)crti.s.o
# TODO
#CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

# The list of assembly objects
ASM_OBJ := $(patsubst $(SRC_DIR)%.s, $(OBJ_DIR)%.s.o, $(ASM_SRC))
# The list of C language objects
C_OBJ := $(patsubst $(SRC_DIR)%.c, $(OBJ_DIR)%.c.o, $(C_SRC))

# TODO
#CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
# TODO
#CRTN_OBJ = $(OBJ_DIR)crtn.s.o

# The list of objects
OBJ := $(ASM_OBJ) $(C_OBJ)
# Object files that are result of code which is part of the sources
#INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
# TODO
#OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

# Cargo
CARGO = cargo +nightly
# Cargo flags
CARGOFLAGS = --verbose
ifeq ($(CONFIG_DEBUG), false)
CARGOFLAGS += --release
endif
ifeq ($(KERNEL_TEST), true)
CARGOFLAGS = --tests
endif

# The Rust language compiler flags
RUSTFLAGS = -Zmacro-backtrace -C link-arg=-T$(LINKER) $(CONFIG_ARGS)

# The strip program
STRIP = strip

# The list of Rust language source files
RUST_SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")



ifeq ($(CONFIG_EXISTS), 0)
# The rule to compile everything
all: $(NAME) iso tags
else
all:
	echo "File $(CONFIG_FILE) doesn't exist. Use \`make config\` to create it"

.SILENT: all
endif

# The rule to create object directories
$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

# The rule to build the library
$(LIB_NAME): $(OBJ_DIRS) $(OBJ)
	$(AR) $(ARFLAGS) $@ $(OBJ)

# The rule to compile assembly objects
$(OBJ_DIR)%.s.o: $(SRC_DIR)%.s $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to compile C language objects
$(OBJ_DIR)%.c.o: $(SRC_DIR)%.c $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

$(NAME): $(LIB_NAME) $(RUST_SRC) $(LINKER) Makefile
	RUSTFLAGS='$(RUSTFLAGS)' $(CARGO) build $(CARGOFLAGS) --target $(TARGET)
ifeq ($(CONFIG_DEBUG), false)
	cp target/target/release/maestro .
	$(STRIP) $(NAME)
else
	cp `ls -1 target/target/debug/deps/maestro-* | head -n 1` $@
endif

# Alias for $(NAME).iso
iso: $(NAME).iso

# The rule to compile the .iso file image, using grub as a bootloader
$(NAME).iso: $(NAME) grub.cfg
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

# The rule to clean the workspace
clean:
	rm -rf $(OBJ_DIR)
	rm -rf $(LIB_NAME)
	rm -f tags
	rm -rf iso/

# The rule to clean the workspace, including target binaries
fclean: clean
	rm -rf target/
	rm -f $(NAME)
	rm -f $(NAME).iso
	rm -rf $(DOC_DIR)
	rm -rf config/target/

# The rule to recompile everything
re: fclean all



# Runs the configuration utility to create the configuration file
config:
	cd config/ && cargo build --release
	config/target/release/config



# The rule to create the `tags` file
tags: $(SRC) $(HDR) $(RUST_SRC)
	ctags $(SRC) $(HDR) $(RUST_SRC)



# Flags for the QEMU emulator
QEMU_FLAGS = -cdrom $(NAME).iso -device isa-debug-exit,iobase=0xf4,iosize=0x04

# The rule to test the kernel using QEMU
test: iso
	qemu-system-i386 $(QEMU_FLAGS) -d int

# The rule to run a CPU test of the kernel using QEMU (aka running the kernel and storing a lot of
# logs into the `cpu_out` file)
cputest: iso
	qemu-system-i386 $(QEMU_FLAGS) -d int,cpu >cpu_out 2>&1

# The rule to test the kernel using Bochs. The configuration for Bochs can be found in the file
# `.bochsrc`
bochs: iso
	bochs

# The rule to run virtualbox
virtualbox: iso
	virtualbox



# The path to the documentation sources
DOC_SRC_DIR = doc_src/
# The path to the documentation build directory
DOC_DIR = doc/

# Builds the documentation
doc: $(DOC_SRC_DIR)
	sphinx-build $(DOC_SRC_DIR) $(DOC_DIR)



# Runs clippy on the Rust code
clippy:
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) clippy $(CARGOFLAGS) --target $(TARGET)



.PHONY: check_config all iso clean fclean re config test debug bochs doc clippy
