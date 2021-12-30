use uefi::table::boot::{BootServices, MemoryDescriptor, MemoryType};

use core::fmt::Debug;

use alloc::vec::Vec;

#[repr(C)]
#[derive(Debug)]
enum MemoryKind {
    Reclaim,
    ACPIReclaim,
    ACPINonVolatile,
}

#[repr(C)]
#[derive(Debug)]
pub struct MemoryEntry {
    start: usize,
    end: usize,
    kind: MemoryKind,
}

impl MemoryEntry {
    pub fn from_mem_desc(memd: &MemoryDescriptor) -> Self {
        MemoryEntry {
            start: Self::align(memd.phys_start as usize, 4096),
            end: Self::align(
                (memd.phys_start + (memd.page_count * 4096 - 1)) as usize,
                4096,
            ),
            kind: if memd.ty == MemoryType::ACPI_NON_VOLATILE {
                MemoryKind::ACPINonVolatile
            } else if memd.ty == MemoryType::ACPI_RECLAIM {
                MemoryKind::ACPIReclaim
            } else {
                MemoryKind::Reclaim
            },
        }
    }

    fn align(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
}

pub fn get_memory_map(bt: &BootServices) -> Vec<MemoryDescriptor> {
    let (buffer, size) = mmap_buffer(bt);
    let mut unit_buffer = unsafe { core::slice::from_raw_parts_mut(buffer, size) };
    let (_mmap_key, mmap_iter) = die_if_failure(bt.memory_map(&mut unit_buffer));
    assert!(mmap_iter.len() > 0, "Memory map is empty");

    let entries: Vec<MemoryDescriptor> = mmap_iter
        .copied()
        /*
        .filter(|i: &MemoryDescriptor| {
            i.ty != MemoryType::RUNTIME_SERVICES_CODE
                && i.ty != MemoryType::RUNTIME_SERVICES_DATA
                && i.ty != MemoryType::ACPI_NON_VOLATILE
                && i.ty != MemoryType::PAL_CODE
        })
        */
        .filter(|i: &MemoryDescriptor| {
            i.ty == MemoryType::BOOT_SERVICES_CODE
                || i.ty == MemoryType::BOOT_SERVICES_DATA
                || i.ty == MemoryType::CONVENTIONAL
                || i.ty == MemoryType::ACPI_RECLAIM
                || i.ty == MemoryType::ACPI_NON_VOLATILE
        })
        .collect();

    assert_eq!(entries[0].phys_start, 0, "Memory doesn't start at 0");
    assert_ne!(entries[0].page_count, 0, "Memory map entry has zero size");

    entries
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
