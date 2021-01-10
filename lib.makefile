# This makefile is meant to build the C and assembly code present in the project in the form of libraries to be linked
# to the Rust code.



# The name of the library to build.
LIB_NAME ?= maestro

# The filename for the library.
NAME = lib$(LIB_NAME).a

# The root of the project with C code to build. The makefile will append `src/` at the end of it to search for the
# sources.
BUILD_ROOT ?= ./

# The C debug flags to use
DEBUG_FLAGS = -D KERNEL_DEBUG -D KERNEL_DEBUG_SANITY -D KERNEL_SELFTEST #-D KERNEL_DEBUG_SPINLOCK

# The C language compiler
CC ?= i686-elf-gcc
# The C language compiler flags
CFLAGS = -nostdlib -ffreestanding -fstack-protector-strong -fno-pic -mno-red-zone -Wall -Wextra -Werror -lgcc
ifeq ($(KERNEL_MODE), release)
CFLAGS += -O3
else
CFLAGS += -g3 $(DEBUG_FLAGS)
endif

# The archive creator program
AR = ar
# The archive creator program flags
ARFLAGS = rc

# The directory containing sources
SRC_DIR = $(BUILD_ROOT)/src/
# The directory containing object files to link
OBJ_DIR = $(BUILD_ROOT)/obj/

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

# The rule to compile everything
all: $(NAME)

# The rule to build the library
$(NAME): $(OBJ_DIRS) $(INTERNAL_OBJ)
	$(AR) $(ARFLAGS) $@ $(OBJ)

# The rule to create object directories
$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

# The rule to compile assembly objects
$(OBJ_DIR)%.s.o: $(SRC_DIR)%.s $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to compile C language objects
$(OBJ_DIR)%.c.o: $(SRC_DIR)%.c $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

# The rule to create the `tags` file
tags: $(SRC) $(HDR)
	ctags $(SRC) $(HDR)

# The rule to clean the workspace
clean:
	rm -rf $(OBJ_DIR)
	rm -f tags

# The rule to clean the workspace, including target binaries
fclean: clean
	rm -f $(NAME)
	rm -rf target/
	rm -f $(NAME).iso

# The rule to recompile everything
re: fclean all

.PHONY: all clean fclean re
