use uefi::table::boot::{BootServices, MemoryDescriptor, MemoryType};

use log::{info, warn};

use core::{arch::asm, fmt::Debug};

use alloc::vec::Vec;

pub fn get_memory_map(bt: &BootServices) {
    let (mut buffer, size) = mmap_buffer(bt);
    let mut unit_buffer = unsafe { core::slice::from_raw_parts_mut(buffer, size) };
    let (_mmap_key, mmap_iter) = die_if_failure(bt.memory_map(&mut unit_buffer));
    assert!(mmap_iter.len() > 0, "Memory map is empty");
    let mut md_buffer: &[MemoryDescriptor] = unsafe {
        core::slice::from_raw_parts(
            core::mem::transmute::<*mut u8, *mut MemoryDescriptor>(buffer),
            size,
        )
    };

    let entries: Vec<MemoryDescriptor> = md_buffer
        .iter()
        .copied()
        .filter(|i: &MemoryDescriptor| {
            i.ty != MemoryType::RUNTIME_SERVICES_CODE
                && i.ty != MemoryType::RUNTIME_SERVICES_DATA
                && i.ty != MemoryType::ACPI_NON_VOLATILE
                && i.ty != MemoryType::PAL_CODE
        })
        .collect();

    assert_eq!(entries[0].phys_start, 0, "Memory doesn't start at 0");
    assert_ne!(entries[0].page_count, 0, "Memory map entry has zero size");

    for i in 0..entries.len() {
        if i + 1 >= entries.len() {
            info!("Type: {:?}, {}-END", entries[i].ty, entries[i].phys_start)
        } else {
            info!(
                "Type: {:?}, {}-{}",
                entries[i].ty,
                entries[i].phys_start,
                entries[i + 1].phys_start - 1
            );
        };
    }
}

pub fn mmap_buffer(bt: &BootServices) -> (*mut u8, usize) {
    let map_sz = bt.memory_map_size();

    let buf_sz = map_sz + 2 * core::mem::size_of::<MemoryDescriptor>();

    (vec![0_u8; buf_sz].as_ptr() as *mut u8, map_sz)
}

pub fn die_if_failure<T, E>(res: uefi::Result<T, E>) -> T
where
    E: Debug,
{
    match res {
        Ok(v) => v.log(),
        Err(e) => panic!("{:?}", e),
    }
}
