UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    EXT_SUFFIX := dylib
else ifeq ($(UNAME_S),Linux)
    EXT_SUFFIX := so
else
    EXT_SUFFIX := dll
endif
EXT_PATH := target/release/libphprs_hello_world.$(EXT_SUFFIX)

build:
	cargo build --release

rusttest:
	cargo test

phptest:
	php -d extension=$(EXT_PATH) vendor/bin/phpunit

test: build rusttest phptest

bench: build
	php -d extension=$(EXT_PATH) vendor/bin/phpbench run --report=aggregate

.PHONY: build rusttest phptest test bench
