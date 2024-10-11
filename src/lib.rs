#![no_std]
#![feature(const_option)]
#![feature(allocator_api)]
mod fs;
mod logger;
mod mem;
mod sd;
mod timer;
mod uart;

use core::slice;

use alloc::string::ToString;
use fs::Volume;
use gpt::{GptLayout, PRIMARY_HEADER_LBA};
use log::info;
use minifat::{FileSystem, FsOptions, NullTimeProvider, Read};
pub use uart::*;
extern crate alloc;
const LOADER_NAME: &str = "LOADER.EFI";

pub fn init(code_end: usize) {
    uart::init();
    logger::init(log::Level::Info);
    sd::init();
    mem::init(code_end);
    info!("environment initialized");
}
pub fn load_bootloader(load_addr: usize) -> usize {
    let mut buf = [0u8; 512];
    let mut gpt = GptLayout::new();
    let part_index = find_efi_partition(&mut gpt, &mut buf);
    let part = gpt.partition(part_index).unwrap();
    let mut fs = init_fat(part.start_lba as usize, part.end_lba as usize);
    load_loader(&mut fs, load_addr)
}

fn find_efi_partition(gpt: &mut GptLayout, blk: &mut [u8]) -> usize {
    let efi_uuid = "c12a7328-f81f-11d2-ba4b-00c93ec90";
    info!("find efi partition...");
    sd::read_block(PRIMARY_HEADER_LBA, blk);
    gpt.init_primary_header(blk).unwrap();
    let part_start = gpt.primary_header().part_start as usize;
    sd::read_block(part_start, blk);
    gpt.init_partitions(blk, 1);
    let efi_part = gpt.partition(3).unwrap();
    if efi_part.part_type_guid.to_string().eq(efi_uuid) {
        info!("find efi partition {}", 3);
    }
    3
}

fn init_fat(start_lba: usize, end_lba: usize) -> FileSystem<Volume, NullTimeProvider> {
    info!("init fat file system");
    FileSystem::new(
        Volume::new(start_lba, end_lba, unsafe { sd::blk_dev_mut() }),
        FsOptions::new(),
    )
    .unwrap()
}

fn load_loader(fs: &mut FileSystem<Volume, NullTimeProvider>, load_addr: usize) -> usize {
    let root = fs.root_dir();
    let mut size = 0;
    for item in root.iter() {
        let entry = item.unwrap();
        if entry.is_file() && entry.short_file_name().eq(LOADER_NAME) {
            info!("load boot loader program {}", entry.short_file_name());
            let mut file = entry.to_file();
            size = file.size().unwrap() as usize;
            let buf = unsafe { slice::from_raw_parts_mut(load_addr as *mut u8, size) };
            file.read_exact(buf).unwrap();
            info!("boot loader program load success");
            break;
        }
    }
    size
}
