# This makefile builds the configuration utility

SRC_DIR = src/
SRC := $(shell find $(SRC_DIR) -type f -name "*.rs")

PATH = target/release/config

# Builds everything
all: $(PATH)

$(PATH): $(SRC)
	cargo build --release

# Cleans the directory
fclean:
	rm -rf target/

.PHONY: all fclean
