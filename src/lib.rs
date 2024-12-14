#![no_std]
mod fat;
mod logger;
mod mem;
mod sd;
mod uart;

use core::{ops::Deref, slice};

use fat::Volume;
use gpt::{GptLayout, Partition, PRIMARY_HEADER_LBA};
use log::{error, info};
pub use uart::*;
extern crate alloc;

// EFI GUID: C12A7328-F81F-11D2-BA4B-00A0C93EC93B
const EFI_GUID: [u8; 16] = [
    0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
];

pub fn init(code_end: usize) {
    uart::init_uart();
    logger::init(log::Level::Info);
    info!("logger init success");
    sd::init();
    mem::init(code_end);
    info!("Vision five 2 firmware, environment initialized");
}

pub fn load_kernel(load_addr: usize, kernel_name: &str) {
    let volume = find_efi_partition().map_or_else(
        || {
            panic!("can not found an efi partition");
        },
        |efi_part| {
            info!("init fat filesystem for parrition: \n{}", efi_part);
            init_fat(efi_part.start_lba as usize)
        },
    );
    if let Some((lba, size)) = volume.find(kernel_name, unsafe { sd::blk_dev_mut() }) {
        load_to_mem(lba, size, load_addr);
    } else {
        error!("Can not find kernel {}", kernel_name)
    }
}

/// 列出SD卡中的前四个分区
pub fn find_efi_partition() -> Option<Partition> {
    info!("find efi partition");
    let mut buf = [0u8; 512];
    let mut gpt = GptLayout::new();
    sd::read_block(PRIMARY_HEADER_LBA, &mut buf);
    gpt.init_primary_header(&buf).unwrap();
    let part_start = gpt.primary_header().part_start as usize;
    sd::read_block(part_start, &mut buf);
    let mut efi_partition = None;
    let part_entry_size = 128;
    for index in 0..4 {
        let start = part_entry_size * index;
        let end = start + part_entry_size;
        let entry_index = index + 1;
        gpt.init_partition(&buf[start..end], entry_index);
        if let Some(part) = gpt.partition(entry_index) {
            info!(
                "Partition {entry_index}: {},{}",
                part.name, part.part_type_guid
            );
            let guid = part.part_type_guid.deref();
            if guid.eq(&EFI_GUID) {
                efi_partition = Some(part.clone());
            }
        }
    }
    efi_partition
}

fn init_fat(start_lba: usize) -> Volume {
    let mut bpb = [0u8; 512];
    sd::read_block(start_lba, &mut bpb[..]);
    let mut volume = Volume::new(start_lba);
    volume.init_bpb(&bpb);
    volume
}

fn load_to_mem(lba: usize, size: usize, load_addr: usize) {
    info!(
        "loading kernel to memory, and the loading address is {:x}",
        load_addr
    );
    let blocks = if size % 512 == 0 {
        size / 512
    } else {
        size / 512 + 1
    };
    for blk_idx in 0..blocks {
        let block_lba = blk_idx + lba;
        let buf = unsafe {
            let ptr = (load_addr as *mut u8).add(blk_idx * 512);
            slice::from_raw_parts_mut(ptr, 512)
        };
        sd::read_block(block_lba, buf);
    }
    info!("kernel load success, and loader size is {}", size);
}
