use uefi::table::boot::{BootServices, MemoryDescriptor, MemoryType};

use core::fmt::Debug;

use alloc::vec::Vec;

pub fn get_memory_map(bt: &BootServices) -> Vec<MemoryDescriptor> {
    let map_sz = bt.memory_map_size();

    let buf_sz = map_sz + 2 * core::mem::size_of::<MemoryDescriptor>();

    let mut buffer = vec![0_u8; buf_sz];
    let (_mmap_key, mmap_iter) = die_if_failure(bt.memory_map(&mut buffer));
    assert!(mmap_iter.len() > 0, "Memory map is empty");

    /*
    let entries_r = mmap_iter.copied().collect::<Vec<MemoryDescriptor>>();
    log::info!("{:#?}", entries_r);
    */

    let entries: Vec<MemoryDescriptor> = mmap_iter
        // .iter()
        .filter(|i| {
            i.ty == MemoryType::BOOT_SERVICES_CODE
                || i.ty == MemoryType::BOOT_SERVICES_DATA
                || i.ty == MemoryType::CONVENTIONAL
                || i.ty == MemoryType::ACPI_RECLAIM
                || i.ty == MemoryType::ACPI_NON_VOLATILE
        })
        .map(|i| *i)
        .collect();

    assert_eq!(entries[0].phys_start, 0, "Memory doesn't start at 0");
    assert_ne!(entries[0].page_count, 0, "Memory map entry has zero size");

    entries
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
