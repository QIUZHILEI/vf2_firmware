#!/bin/bash
cur_dir=`pwd`
export PATH=$PATH:$cur_dir/tools
bin_name="fw.bin"
out_name="fw.img"
rm $bin_name $out_name
cargo +nightly build --release --target riscv64gc-unknown-none-elf
riscv64-unknown-elf-objcopy ./target/riscv64gc-unknown-none-elf/release/vf2_firmware -O binary $bin_name
exec vf2-imager -i $bin_name -o $out_name