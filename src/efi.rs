use log::info;

use crate::utility::mmap_buffer;

use uefi::{
    prelude::{Boot, SystemTable},
    Handle, ResultExt,
};

/*
pub fn parse_and_load_efi(st: &mut SystemTable<Boot>, elf_buf: &[u8]) -> *const u8 {
    let elf_s = Elf::parse(elf_buf).expect("FAILED: COULDN'T PARSE ELF");
    let headers = elf_s
        .program_headers
        .iter()
        .filter(|i| i.p_type == 1)
        .collect::<Vec<&ProgramHeader>>();
    let to_sub = headers[0].p_vaddr;
    let size = headers[headers.len() - 1].p_vaddr - to_sub;
    let pointer = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            size as usize / 4096,
        )
        .expect_success("FAILED: COULDN'T ALLOCATE MEMORY");
    let memory = unsafe { core::slice::from_raw_parts_mut(pointer as *mut u8, size as usize) };
    memory.fill(0);
    info!("Starting address = {:#?}, Size: {}", memory.as_ptr(), size);

    // info!("HEADER: {:#?}\nHEADERS TO LOAD: {:#?}", elf_s.header, headers);

    for header in headers.iter() {
        let index = header.p_vaddr - to_sub;
        let data = &elf_buf[header.p_offset as usize..][..header.p_filesz as usize];
        info!(
            "Writing V_ADDR {}, INDEX: {}, LEN: {}",
            header.p_vaddr,
            index,
            data.len()
        );
        memory[index as usize..][..data.len()].copy_from_slice(data);
        info!(
            "Wrote V_ADDR {}, INDEX: {}, LEN: {}",
            header.p_vaddr,
            index,
            data.len()
        );
    }

    info!("LOADED ELF");

    let entry = elf_s.header.e_entry - to_sub;
    let entry_pointer = unsafe { memory.as_ptr().offset(entry as isize) };
    info!(
        "TO_SUB: 0x{:x}, ENTRY: 0x{:x}, PTR: {:#?}",
        to_sub, entry, entry_pointer
    );
    entry_pointer
}
*/

pub fn exit_boot_services(st: SystemTable<Boot>, image: Handle) {
    info!("EXITING BOOT SERVICES");
    let mut buffer = mmap_buffer(st.boot_services());
    let (_st, _mmap) = st
        .exit_boot_services(image, &mut buffer)
        .expect_success("FAILED: BOOT SERVICES NOT EXITED");
}

macro_rules! load_efi {
    ($buf:ident, $st:ident) => {{
        let elf_buf = $buf;
        let st = unsafe { $st.unsafe_clone() };
        let elf_s = goblin::elf::Elf::parse(&elf_buf).expect("FAILED: COULDN'T PARSE ELF");
        let headers = elf_s
            .program_headers
            .iter()
            .filter(|i| i.p_type == 1)
            .collect::<Vec<&goblin::elf::ProgramHeader>>();
        let to_sub = headers[0].p_vaddr;
        let mut highest = 0;
        let _ = headers
            .iter()
            .cloned()
            .map(|x| {
                let y = x.p_vaddr + x.p_memsz;
                if y > highest {
                    highest = y;
                }
            })
            .collect::<Vec<_>>();
        let size = highest - to_sub;
        let pointer = st
            .boot_services()
            .allocate_pages(
                uefi::table::boot::AllocateType::AnyPages,
                uefi::table::boot::MemoryType::LOADER_DATA,
                ((size + (4096 - 1)) / 4096 + 4) as usize,
            )
            .expect_success("FAILED: COULDN'T ALLOCATE MEMORY");
        let n_pointer = unsafe { (pointer as *mut u8).offset(4 * 4096) };
        let memory = unsafe { core::slice::from_raw_parts_mut(n_pointer, size as usize) };
        memory.fill(0);
        // info!("Starting address = {:#?}, Size: {}", memory.as_ptr(), size);

        /*
        info!(
            "HEADER: {:#?}\nHEADERS TO LOAD: {:#?}",
            elf_s.header, headers
        );
        */

        for header in headers.iter() {
            // info!("Header: {:#?}", header);

            let index = header.p_vaddr - to_sub;

            let src = &elf_buf[header.p_offset as usize..][..header.p_filesz as usize];

            let dest = &mut memory[index as usize..][..src.len()];

            dest.copy_from_slice(src);
        }

        info!("LOADED ELF");

        let entry = elf_s.header.e_entry - to_sub;
        unsafe {
            (
                memory.as_ptr().offset(-1),
                memory.as_ptr().offset(entry as isize),
            )
        }
    }};
}
pub(crate) use load_efi;
