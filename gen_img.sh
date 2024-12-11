#!/bin/bash
export PATH=$PATH:$(pwd)/bin
cargo +nightly build --release --target riscv64gc-unknown-none-elf
riscv64-unknown-elf-objcopy ./target/riscv64gc-unknown-none-elf/release/vf2_firmware -O binary fw.bin
vf2-imager -i fw.bin -o fw.img