NAME = kernel

CC = i686-elf-gcc
CA = i686-elf-as

CFLAGS = -ffreestanding -O2 -Wall -Wextra -Werror
LINK_FLAGS = -nostdlib -lgcc

LINKER = linker.ld

ASM_SRC := $(shell find src/ -type f -name "*.c")
C_SRC := $(shell find src/ -type f -name "*.s")

ASM_OBJ := $(patsubst %.s, %.o, $(ASM_SRC))
C_OBJ := $(patsubst %.c, %.o, $(C_SRC))

OBJ := $(ASM_OBJ) $(C_OBJ)

$(NAME): $(OBJ)
	echo $(OBJ)
	#$(CC) $(CFLAGS) $(LINK_FLAGS) -T $(LINKER) -o $(NAME) $(OBJ)

$(ASM_OBJ): $(ASM_SRC)
	$(CA) -c $< -o $@

$(C_OBJ): $(C_SRC)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -rf $(OBJ)

fclean: clean
	rm -rf $(NAME)

re: fclean all
