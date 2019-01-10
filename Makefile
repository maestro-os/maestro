NAME = crumbleos

CC = i686-elf-gcc
CFLAGS = -ffreestanding -O2 -Wall -Wextra -Werror -nostdlib -lgcc -g

LINKER = linker.ld

ASM_SRC := $(shell find src/ -type f -name "*.s" -and ! -name "crti.s" -and ! -name "crtn.s")
C_SRC := $(shell find src/ -type f -name "*.c")

SRC := $(ASM_SRC) $(C_SRC)

CRTI_OBJ = src/crti.o
CRTBEGIN_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtbegin.o)

ASM_OBJ := $(patsubst %.s, %.o, $(ASM_SRC))
C_OBJ := $(patsubst %.c, %.o, $(C_SRC))

CRTEND_OBJ := $(shell $(CC) $(CFLAGS) -print-file-name=crtend.o)
CRTN_OBJ = src/crtn.o

OBJ := $(ASM_OBJ) $(C_OBJ) 
INTERNAL_OBJ := $(CRTI_OBJ) $(OBJ) $(CRTN_OBJ)
OBJ_LINK_LIST := $(CRTI_OBJ) $(CRTBEGIN_OBJ) $(OBJ) $(CRTEND_OBJ) $(CRTN_OBJ)

all: $(NAME) iso

$(NAME): $(INTERNAL_OBJ)
	$(CC) $(CFLAGS) -T $(LINKER) -o $(NAME) $(OBJ_LINK_LIST)

%.o: %.[cs]
	$(CC) $(CFLAGS) -c $< -o $@

iso: $(NAME).iso

$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

tags:
	ctags -R src/ --languages=c,c++

clean:
	rm -f $(INTERNAL_OBJ)
	rm -rf iso

fclean: clean
	rm -f $(NAME)
	rm -f $(NAME).iso

re: fclean all

test: $(NAME)
	qemu-system-i386 -cdrom $(NAME).iso -d guest_errors

debug: $(NAME)
	qemu-system-i386 -cdrom $(NAME).iso -s -S

.PHONY: all iso clean fclean re test debug
