NAME = maestro

ARCH ?= x86

TARGET = arch/$(ARCH)/target.json
LINKER = arch/$(ARCH)/linker.ld

DEBUG_FLAGS = -D KERNEL_DEBUG -D KERNEL_DEBUG_SANITY -D KERNEL_SELFTEST #-D KERNEL_DEBUG_SPINLOCK

CC = i686-elf-gcc
CFLAGS = -nostdlib -ffreestanding -fstack-protector-strong -mno-red-zone -Wall -Wextra -Werror -lgcc -g3 $(DEBUG_FLAGS)

RUST = rustc
RUSTFLAGS = --emit=obj --target=$(TARGET)

SRC_DIR = src/
OBJ_DIR = obj/

ASM_SRC := $(shell find $(SRC_DIR) -type f -name "*.s" -and ! -name "crti.s" -and ! -name "crtn.s")
C_SRC := $(shell find $(SRC_DIR) -type f -name "*.c")
HDR := $(shell find $(SRC_DIR) -type f -name "*.h")
RUST_SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")

DIRS := $(shell find $(SRC_DIR) -type d)
OBJ_DIRS := $(patsubst $(SRC_DIR)%, $(OBJ_DIR)%, $(DIRS))

SRC := $(ASM_SRC) $(C_SRC) $(RUST_SRC)

CRTI_OBJ = $(OBJ_DIR)crti.s.o
CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

ASM_OBJ := $(patsubst $(SRC_DIR)%.s, $(OBJ_DIR)%.s.o, $(ASM_SRC))
C_OBJ := $(patsubst $(SRC_DIR)%.c, $(OBJ_DIR)%.c.o, $(C_SRC))
RUST_OBJ := $(patsubst $(SRC_DIR)%.rs, $(OBJ_DIR)%.rs.o, $(RUST_SRC))

CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
CRTN_OBJ = $(OBJ_DIR)crtn.s.o

OBJ := $(ASM_OBJ) $(C_OBJ) $(RUST_OBJ)
INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

LIBCORE = rust/libcore.rlib

all: tags $(NAME) iso

$(NAME): $(OBJ_DIRS) $(INTERNAL_OBJ) $(LINKER)
	$(CC) $(CFLAGS) --gc-sections -T $(LINKER) -o $(NAME) $(OBJ_LINK_LIST)

$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

$(OBJ_DIR)%.s.o: $(SRC_DIR)%.s $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

$(OBJ_DIR)%.c.o: $(SRC_DIR)%.c $(HDR) Makefile
	$(CC) $(CFLAGS) -I $(SRC_DIR) -c $< -o $@

$(OBJ_DIR)%.rs.o: $(SRC_DIR)%.rs $(HDR) $(LIBCORE) Makefile $(TARGET)
	$(RUST) $(RUSTFLAGS) -o $@ --extern core=$(LIBCORE) $<

$(LIBCORE):
	make libcore -C rust/

iso: $(NAME).iso

$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

tags: $(SRC) $(HDR)
	ctags $(SRC) $(HDR)

clean:
	rm -rf obj/
	rm -rf iso/
	rm -f tags

fclean: clean
	make fclean -C rust/
	rm -f $(NAME)
	rm -f $(NAME).iso

re: fclean all

test: iso
	qemu-system-i386 -cdrom $(NAME).iso -d int

cputest: iso
	qemu-system-i386 -cdrom $(NAME).iso -d int,cpu >cpu_out 2>&1

bochs: iso
	bochs

virtualbox: iso
	virtualbox

.PHONY: all iso clean fclean re test debug bochs
