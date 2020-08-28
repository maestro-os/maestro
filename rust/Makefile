URL = https://static.rust-lang.org/dist/rustc-nightly-src.tar.gz
ARCHIVE = rustc-nightly-src.tar.gz
DIR = rustc-nightly-src

TARGET = ../arch/x86/target.json

CARGO = cargo
CARGOFLAGS = -Z unstable-options
RUSTC = rustc
RUSTFLAGS = -Z unstable-options

LIBCORE_PATH = $(DIR)/library/core/
LIBCORE = libcore.rlib

LIBCOMPILER_BUILTINS_PATH = $(DIR)/vendor/compiler_builtins/
LIBCOMPILER_BUILTINS = libcompiler_builtins.rlib

all: $(LIBCORE) $(LIBCOMPILER_BUILTINS)

clean:
	rm -rf $(DIR)
	rm -f core.d
	rm -f $(LIBCORE)

fclean: clean
	rm -f $(ARCHIVE)

update: fclean all

$(ARCHIVE):
	curl $(URL) -o $@

$(DIR): $(ARCHIVE)
	tar -xf $(ARCHIVE)
	touch $(DIR)

$(LIBCORE): $(DIR)
	cd "$(LIBCORE_PATH)" && $(CARGO) build $(CARGOFLAGS) --target ../../../$(TARGET) --out-dir $(shell pwd)

$(LIBCOMPILER_BUILTINS): $(DIR)
	$(RUSTC) $(RUSTFLAGS) --out-dir $(shell pwd) --crate-name=compiler_builtins --crate-type=lib --target $(TARGET) --extern core=$(LIBCORE) $(LIBCOMPILER_BUILTINS_PATH)/src/lib.rs --cfg stage0 --cfg feature='"compiler-builtins"' --emit=link,dep-info

.PHONY: all clean update
