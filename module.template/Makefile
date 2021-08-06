# This Makefile builds the kernel module

# The name of the module
NAME = <NAME>
# The module's final file
MOD_FILE = $(NAME).kmod

# The path to the kernel's sources
KERN_SRC ?=

# Configuration as arguments for the compiler
CONFIG_ARGS := $(shell cd $(KERN_SRC) && scripts/config_args.sh)
# Configuration as arguments as environment variables
CONFIG_ENV := BUILD_MODULE=true $(shell cd $(KERN_SRC) && scripts/config_env.sh)

# The architecture to compile for
CONFIG_ARCH ?= $(shell cd $(KERN_SRC) && scripts/config_attr.sh general_arch)
# Tells whether to compile in debug mode
CONFIG_DEBUG := $(shell cd $(KERN_SRC) && scripts/config_attr.sh debug_debug)

# The absolute path to the target file
TARGET_PATH := $(shell realpath "$(KERN_SRC)/arch/$(CONFIG_ARCH)/target.json")

# The flags for Cargo
CARGOFLAGS = --target $(TARGET_PATH)
ifeq ($(CONFIG_DEBUG), false)
CARGOFLAGS += --release
endif

# The flags for the Rust compiler
RUSTFLAGS = -Zmacro-backtrace -C prefer-dynamic $(CONFIG_ARGS)

# TODO Error if kern_src is not set
# TODO Error if selftest is enabled

$(MOD_FILE):
	$(CONFIG_ENV) RUSTFLAGS='$(RUSTFLAGS)' cargo +nightly build $(CARGOFLAGS)
ifeq ($(CONFIG_DEBUG), true)
	cp target/target/debug/lib$(NAME).so $(MOD_FILE)
else
	cp target/target/release/lib$(NAME).so $(MOD_FILE)
endif

.PHONY: $(MOD_FILE)
