NAME = crumbleos

CC = i686-elf-gcc
CFLAGS = -nostdlib -ffreestanding -fstack-protector-strong -Wall -Wextra -Werror -O2 -lgcc -g -D KERNEL_DEBUG

LINKER = linker.ld

ASM_SRC := $(shell find src/ -type f -name "*.s" -and ! -name "crti.s" -and ! -name "crtn.s")
C_SRC := $(shell find src/ -type f -name "*.c")
HDR := $(shell find src/ -type f -name "*.h")

DIRS := $(shell find src/ -type d)
OBJ_DIRS := $(patsubst src/%, obj/%, $(DIRS))

SRC := $(ASM_SRC) $(C_SRC)

CRTI_OBJ = obj/crti.s.o
CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

ASM_OBJ := $(patsubst src/%.s, obj/%.s.o, $(ASM_SRC))
C_OBJ := $(patsubst src/%.c, obj/%.c.o, $(C_SRC))

CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
CRTN_OBJ = obj/crtn.s.o

OBJ := $(ASM_OBJ) $(C_OBJ) 
INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

all: tags $(NAME) iso

$(NAME): $(OBJ_DIRS) $(INTERNAL_OBJ)
	$(CC) $(CFLAGS) -T $(LINKER) -o $(NAME) $(OBJ_LINK_LIST)

$(OBJ_DIRS):
	mkdir -p $(OBJ_DIRS)

obj/%.s.o: src/%.s $(HDR)
	$(CC) $(CFLAGS) -c $< -o $@

obj/%.c.o: src/%.c $(HDR)
	$(CC) $(CFLAGS) -c $< -o $@

iso: $(NAME).iso

$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

tags:
	ctags -R --languages=c,c++ src/

clean:
	rm -rf obj/
	rm -rf iso/
	rm -f tags

fclean: clean
	rm -f $(NAME)
	rm -f $(NAME).iso

re: fclean all

test: iso
	qemu-system-i386 -cdrom $(NAME).iso -d int

debug: iso
	qemu-system-i386 -cdrom $(NAME).iso -s -S

bochs:
	bochs

.PHONY: all iso clean fclean re test debug bochs
