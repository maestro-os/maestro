URL = https://static.rust-lang.org/dist/rustc-nightly-src.tar.gz
ARCHIVE = rustc-nightly-src.tar.gz
DIR = rustc-nightly-src

KERNEL_ARCH ?= x86

TARGET = ../arch/$(KERNEL_ARCH)/target.json

CARGO = cargo
CARGOFLAGS = -Z unstable-options
RUSTC = rustc
RUSTFLAGS = -Z unstable-options -Copt-level=z

LIBCORE_PATH = $(DIR)/library/core/
LIBCORE = libcore.rlib

#LIBCOMPILER_BUILTINS_PATH = $(DIR)/vendor/compiler_builtins/
#LIBCOMPILER_BUILTINS = libcompiler_builtins.rlib

all: $(LIBCORE)# $(LIBCOMPILER_BUILTINS)

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

$(LIBCORE): $(DIR) Makefile
	$(CARGO) build --manifest-path $(LIBCORE_PATH)/Cargo.toml $(CARGOFLAGS) -Z build-std=core --target $(TARGET) --out-dir $(shell pwd)
	#$(RUSTC) $(RUSTFLAGS) --out-dir $(shell pwd) --crate-name=core --crate-type=lib --target $(TARGET) $(LIBCORE_PATH)/src/lib.rs --cfg stage0 --cfg feature='"core"' --emit=link,dep-info 

#$(LIBCOMPILER_BUILTINS): $(DIR) Makefile
#	$(RUSTC) $(RUSTFLAGS) --out-dir $(shell pwd) --crate-name=compiler_builtins --crate-type=lib --target $(TARGET) --extern core=$(LIBCORE) $(LIBCOMPILER_BUILTINS_PATH)/src/lib.rs --cfg stage0 --cfg feature='"compiler-builtins"' --emit=link,dep-info

.PHONY: all clean update
