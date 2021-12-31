use alloc::vec::Vec;
use goblin::elf::*;
use log::info;

macro_rules! allocate_elf_memory {
    ($st:ident, $size:ident, $s_size:expr) => {{
        use uefi::{
            prelude::*,
            table::boot::{AllocateType, MemoryType},
        };
        let p_size = ($size + (4096 - 1)) / 4096;
        let pointer = $st
            .boot_services()
            .allocate_pages(
                AllocateType::AnyPages,
                MemoryType::LOADER_DATA,
                p_size + $s_size,
            )
            .expect_success("FAILED: COULDN'T ALLOCATE MEMORY") as *mut u8;
        info!(
            "Allocated {} pages for elf and {} for stack",
            p_size, $s_size
        );
        info!("Offset: {:?}", pointer);
        unsafe {
            (core::slice::from_raw_parts_mut(pointer, (p_size + $s_size) * 4096)).fill(0);
            let mem: &mut [u8] =
                core::slice::from_raw_parts_mut(pointer.offset($s_size * 4096), p_size * 4096);
            (mem.as_ptr().offset(-1), mem)
        }
    }};
}

pub(crate) use allocate_elf_memory;

pub fn elf_size(elf_buf: &[u8]) -> (Elf, Vec<ProgramHeader>, usize) {
    let elf = Elf::parse(&elf_buf).expect("FAILED: COULDN'T PARSE ELF");
    let headers = elf
        .program_headers
        .iter()
        .filter(|i| i.p_type == 1)
        .map(|i| i.clone())
        .collect::<Vec<ProgramHeader>>();

    let to_sub = headers[0].p_vaddr;

    let mut highest = 0;
    let _ = headers
        .iter()
        .map(|x| {
            let y = x.p_vaddr + x.p_memsz;
            if y > highest {
                highest = y;
            }
        })
        .collect::<Vec<_>>();

    (elf, headers, (highest - to_sub) as usize)
}

pub fn elf_load(elf_buf: &[u8], memory: &mut [u8], elf: Elf, headers: &[ProgramHeader]) -> *mut u8 {
    let to_sub = headers[0].p_vaddr;

    for header in headers.iter() {
        let index = header.p_vaddr - to_sub;

        let src = &elf_buf[header.p_offset as usize..][..header.p_filesz as usize];

        let dest = &mut memory[index as usize..][..src.len()];

        info!(
            "Writing {} bytes from {:?} to {:?}",
            src.len(),
            src.as_ptr(),
            dest.as_ptr()
        );

        dest.copy_from_slice(src);

        info!("Done writing");
    }

    /*
    info!("Dynamic Relocations W/ Addend");
    for reloc in elf.dynrelas.iter() {
        info!("Relocation: {:#?}", reloc);
    }

    info!("Dynamic Relocations W/o Addend");
    for reloc in elf.dynrels.iter() {
        info!("Relocation: {:#?}", reloc);
    }

    info!("PLT Relocations");
    for reloc in elf.pltrelocs.iter() {
        info!("Relocation: {:#?}", reloc);
    }

    info!("Section Relocations (By Section Index)");
    for reloc in elf.shdr_relocs.iter() {
        info!("Relocation: {:#?}", reloc);
    }
    */

    for reloc in elf.dynrelas.iter() {
        let pointer = memory.as_ptr() as i64;
        info!("Appling relocation at offset: {:#x}", reloc.r_offset);
        memory[reloc.r_offset as usize..][..8]
            .copy_from_slice(&((pointer + reloc.r_addend.unwrap()) as u64).to_le_bytes());
    }

    info!("LOADED ELF");

    unsafe {
        memory
            .as_ptr()
            .offset((elf.header.e_entry - to_sub) as isize) as *mut u8
    }
}

macro_rules! load_elf {
    ($buf:ident, $s_size:expr, $st:ident) => {{
        let (elf, headers, size) = crate::elf::elf_size(&$buf);
        let st = &mut $st;
        let (stack, mut elf_data) = crate::elf::allocate_elf_memory!(st, size, $s_size);
        (
            stack,
            crate::elf::elf_load(&$buf, &mut elf_data, elf, &headers),
        )
    }};
}

pub(crate) use load_elf;

/*
macro_rules! exit_boot_services {
    ($st:ident, $image:ident) => {{
        use crate::utility::{MemoryEntry, MemoryKind};
        use log::info;
        use uefi::table::boot::{MemoryDescriptor, MemoryType};

        info!("EXITING BOOT SERVICES");

        let map_sz = $st.boot_services().memory_map_size();

        let buf_sz = map_sz + 2 * core::mem::size_of::<MemoryDescriptor>();

        let mut buffer = vec![0_u8; buf_sz];
        let mut buffer2 =
            vec![MemoryEntry::new(); (buf_sz / core::mem::size_of::<MemoryDescriptor>())];

        let (_st, mmap) = $st
            .exit_boot_services($image, &mut buffer)
            .expect_success("FAILED: BOOT SERVICES NOT EXITED");

        mmap.enumerate()
            .filter(|(index, i)| {
                i.ty == MemoryType::BOOT_SERVICES_CODE
                    || i.ty == MemoryType::BOOT_SERVICES_DATA
                    || i.ty == MemoryType::CONVENTIONAL
                    || i.ty == MemoryType::ACPI_RECLAIM
                    || i.ty == MemoryType::ACPI_NON_VOLATILE
            })
            .map(|(i, item)| buffer2[i] = { MemoryEntry::from_mem_desc(item) });

        buffer2
    }};
}
pub(crate) use exit_boot_services;
*/
