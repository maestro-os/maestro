NAME = kernel

CC = i686-elf-gcc
CA = i686-elf-as

CFLAGS = -ffreestanding -O2 -Wall -Wextra -Werror
LINK_FLAGS = -nostdlib -lgcc

LINKER = linker.ld

ASM_SRC := $(shell find src/ -type f -name "*.s")
C_SRC := $(shell find src/ -type f -name "*.c")

SRC := $(ASM_SRC) $(C_SRC)

ASM_OBJ := $(patsubst %.s, %.o, $(ASM_SRC))
C_OBJ := $(patsubst %.c, %.o, $(C_SRC))

OBJ := $(ASM_OBJ) $(C_OBJ)

$(NAME): $(OBJ)
	$(CC) $(CFLAGS) $(LINK_FLAGS) -T $(LINKER) -o $(NAME) $(OBJ)

$(OBJ): $(SRC)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ)

fclean: clean
	rm -f $(NAME)

re: fclean all
