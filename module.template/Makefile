# This Makefile builds the kernel module

# The name of the module
NAME = <NAME>
# The module's final file
MOD_FILE = $(NAME).kmod

# The path to the kernel's sources
KERN_SRC ?=

# The path to the configuration file
CONFIG_PATH := $(shell realpath "$(KERN_SRC)/.config")

# Configuration as arguments for the compiler
CONFIG_ARGS := $(shell cd $(KERN_SRC) && scripts/config_args.sh) --cfg kern_src="$(KERN_SRC)"

# The architecture to compile for
CONFIG_ARCH ?= $(shell cd $(KERN_SRC) && scripts/config_attr.sh general_arch)
# Tells whether to compile in debug mode
CONFIG_DEBUG := $(shell cd $(KERN_SRC) && scripts/config_attr.sh debug_debug)

# The absolute path to the linker script
LINKER_PATH := $(shell realpath "$(KERN_SRC)/arch/$(CONFIG_ARCH)/linker.ld")
# The absolute path to the target file
TARGET_PATH := $(shell realpath "$(KERN_SRC)/arch/$(CONFIG_ARCH)/target.json")

# The flags for the Rust compiler
RUSTFLAGS = -C link-arg=-T$(LINKER_PATH) -C prefer-dynamic $(CONFIG_ARGS)

# TODO Error if kern_src is not set
# TODO Error if selftest is enabled

$(MOD_FILE):
ifeq ($(CONFIG_DEBUG), true)
	RUSTFLAGS='$(RUSTFLAGS)' cargo +nightly build --target $(TARGET_PATH)
	cp target/target/debug/lib$(NAME).so $(MOD_FILE)
else
	RUSTFLAGS='$(RUSTFLAGS)' cargo +nightly build --target $(TARGET_PATH) --release
	cp target/target/release/lib$(NAME).so $(MOD_FILE)
endif
