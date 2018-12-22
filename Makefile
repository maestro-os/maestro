NAME = kernel

CC = i686-elf-gcc

CFLAGS = -ffreestanding -O2 -Wall -Wextra -Werror
LINK_FLAGS = -nostdlib -lgcc

LINKER = linker.ld

ASM_SRC := $(shell find src/ -type f -name "*.s")
C_SRC := $(shell find src/ -type f -name "*.c")

SRC := $(ASM_SRC) $(C_SRC)

ASM_OBJ := $(patsubst %.s, %.o, $(ASM_SRC))
C_OBJ := $(patsubst %.c, %.o, $(C_SRC))

OBJ := $(ASM_OBJ) $(C_OBJ)

all: $(NAME) iso

$(NAME): $(OBJ)
	$(CC) $(CFLAGS) $(LINK_FLAGS) -T $(LINKER) -o $(NAME) $(OBJ)

%.o: %.[cs]
	$(CC) $(CFLAGS) -c $< -o $@

iso: $(NAME).iso

$(NAME).iso: $(NAME)
	mkdir -p iso/boot/grub
	cp $(NAME) iso/boot
	cp grub.cfg iso/boot/grub
	grub-mkrescue -o $(NAME).iso iso

clean:
	rm -f $(OBJ)
	rm -rf iso

fclean: clean
	rm -f $(NAME)
	rm -f $(NAME).iso

re: fclean all

.PHONY: iso clean fclean re
