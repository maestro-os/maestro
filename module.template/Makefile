# This Makefile builds the kernel module

# The name of the module
NAME = <NAME>
# The module's final file
MOD_FILE = $(NAME).kmod

# The path to the kernel's sources
KERN_SRC ?=

# The architecture to compile for
CONFIG_ARCH ?= $(shell cd $(KERN_SRC) && scripts/config_attr.sh general_arch)
# Tells whether to compile in debug mode
CONFIG_DEBUG := $(shell cd $(KERN_SRC) && scripts/config_attr.sh debug_debug)

# The absolute path to the target file
TARGET_PATH := $(shell realpath "$(KERN_SRC)/arch/$(CONFIG_ARCH)/target.json")

# The flags for the Rust compiler
RUSTFLAGS = -Zmacro-backtrace --crate-type dylib -C prefer-dynamic -L $(KERN_SRC)/target/target/debug -L $(KERN_SRC)/target/target/debug/deps --target $(TARGET_PATH)
ifeq ($(CONFIG_DEBUG), false)
RUSTFLAGS += -C opt-level=3
endif

ifeq ($(KERN_SRC), )
$(error Set the KERN_SRC environment variable with the path to the sources of the kernel)
endif

# TODO Error if selftest is enabled

all: $(MOD_FILE)

$(MOD_FILE):
	rustc src/mod.rs -o $(MOD_FILE) $(RUSTFLAGS)

clean:
	#rm -rf target/

fclean: clean
	rm -f $(MOD_FILE)

re: fclean all

.PHONY: all $(MOD_FILE) clean fclean re
