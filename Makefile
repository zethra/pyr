.PHONY: all

all: libpyr.so

libpyr.so: src/lib.rs
	cargo build
	cp target/debug/libpyr.so .