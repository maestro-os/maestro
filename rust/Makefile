URL = https://static.rust-lang.org/dist/rustc-nightly-src.tar.gz
ARCHIVE = rustc-nightly-src.tar.gz
DIR = rustc-nightly-src

TARGET = ../arch/x86/target.json

CARGO = cargo
CARGOFLAGS = -Z unstable-options

LIBCORE_FILE = $(DIR)/library/core/Cargo.toml
LIBCORE = libcore.rlib

all: $(LIBCORE)

clean:
	rm -rf $(DIR)
	rm -f core.d
	rm -f $(LIBCORE)

fclean: clean
	rm -f $(ARCHIVE)

update: fclean all

$(ARCHIVE):
	curl $(URL) -o $@

$(LIBCORE_FILE): $(ARCHIVE)
	tar -xf $(ARCHIVE)
	touch $(LIBCORE_FILE)

$(LIBCORE): $(LIBCORE_FILE)
	cd "$(DIR)/library/core/" && $(CARGO) build $(CARGOFLAGS) --target ../../../$(TARGET) --out-dir $(shell pwd)

.PHONY: all clean update
