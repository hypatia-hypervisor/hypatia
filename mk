#!/bin/sh
TARGET_PATH=target/x86_64-unknown-none-elf/debug

cargo build -Z build-std=core,alloc --target lib/x86_64-unknown-none-elf.json "$@"
objcopy --input-target=elf64-x86-64 \
        --output-target=elf32-i386 \
        ${TARGET_PATH}/theon \
        ${TARGET_PATH}/theon-32
