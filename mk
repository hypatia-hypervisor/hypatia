#!/bin/sh
cargo build -Z build-std=core,alloc --target lib/x86_64-unknown-none-elf.json "$@"
