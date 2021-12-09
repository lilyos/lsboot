#![no_main]
#![no_std]
#![feature(abi_efiapi, asm, type_name_of_val)]

use core::fmt::Debug;

use uefi::prelude::*;
use uefi::ResultExt;
use uefi::{
    proto::media::{
        file::{File, FileAttribute, FileMode, FileType},
        fs::SimpleFileSystem,
    },
    table::boot::{AllocateType, BootServices, MemoryDescriptor, MemoryType},
};

use goblin::elf::*;

#[macro_use]
extern crate log;

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;

#[entry]
fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).expect_success("FAILED: BOOT SERVICES UNRESPONSIVE");

    st.stdout()
        .reset(false)
        .expect_success("FAILED: STDOUT NOT RESET");

    info!("Loading Lotus");

    get_memory_map(st.boot_services());

    let prop = st
        .boot_services()
        .locate_protocol::<SimpleFileSystem>()
        .expect_success("FAILED: FILESYSTEM NOT FOUND");

    let prop = unsafe { &mut *prop.get() };

    let mut root = prop
        .open_volume()
        .expect_success("FAILED: ROOT DIRECTORY NOT ACCESSED");

    let kernel = root
        .open("EFI\\lotus\\bud", FileMode::Read, FileAttribute::empty())
        .expect_success("FAILED: BUD NOT FOUND")
        .into_type()
        .expect_success("FAILED: BUD NOT A FILE");

    let mut kernel = match kernel {
        FileType::Regular(file) => file,
        _ => panic!("Bud not a file"),
    };

    let mut info_buf = vec![0u8; 128];
    let kernel_size;
    loop {
        match kernel.get_info(&mut info_buf) {
            Ok(v) => {
                    let v: &mut uefi::proto::media::file::FileInfo = v.log();
                    kernel_size = v.file_size();
                    break;
                }
            Err(e) => info_buf = vec![0u8; e.data().unwrap()],
        }
    }

    let mut kernel_buf = vec![0u8; kernel_size.try_into().unwrap()];
    kernel
        .read(&mut kernel_buf)
        .expect_success("FAILED: COULDN'T READ BUD");

    info!("LENGTH OF KERNEL: {}", kernel_buf.len());

    let kernel_buf_clone = kernel_buf.clone();
    let kernel_elf = Elf::parse(&kernel_buf_clone).unwrap();

    // info!("KERNEL ELFs: {:#?}", kernel_elf);

    load_efi(st, image, kernel_elf, &mut kernel_buf);
}

fn load_efi(st: SystemTable<Boot>, image: Handle, elf_s: Elf, elf_buf: &mut [u8]) -> ! {
    let headers = elf_s.program_headers.iter().filter(|i| i.p_type == 1).collect::<Vec<&ProgramHeader>>();
    let to_sub = headers[0].p_vaddr;
    let size = headers[headers.len()-1].p_vaddr - to_sub;
    let pointer = st.boot_services().allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, size as usize / 4096).expect_success("FAILED: COULDN'T ALLOCATE MEMORY"); 
    let memory = unsafe { core::slice::from_raw_parts_mut(pointer as *mut u8, size as usize) };
    memory.fill(0);
    info!("Starting address = {:#?}, Size: {}", memory.as_ptr(), size);

    // info!("HEADER: {:#?}\nHEADERS TO LOAD: {:#?}", elf_s.header, headers);

    for header in headers.iter() {
        let index = header.p_vaddr - to_sub;
        let data = &elf_buf[header.p_offset as usize..][..header.p_filesz as usize];
        info!("Writing V_ADDR {}, INDEX: {}, LEN: {}", header.p_vaddr, index, data.len());
        memory[index as usize..data.len()].copy_from_slice(data);
        info!("Wrote V_ADDR {}, INDEX: {}, LEN: {}", header.p_vaddr, index, data.len());
    }

    info!("LOADED ELF");

    let entry = elf_s.header.e_entry - to_sub;
    let entry_pointer = unsafe { memory.as_ptr().offset(entry as isize) };  
    info!("TO_SUB: 0x{:x}\nENTRY: 0x{:x}\nPTR: {:#?}", to_sub, entry, entry_pointer);

    info!("EXITING BOOT SERVICES");
    info!("LAUNCHING LOTUS");
    let mut buffer = mmap_buffer(st.boot_services());
    let (_st, _mmap) = st
        .exit_boot_services(image, &mut buffer)
        .expect_success("FAILED: BOOT SERVICES NOT EXITED");

    unsafe {
        asm!("jmp {}", in(reg) entry_pointer, options(noreturn));
    }
}

fn get_memory_map(bt: &BootServices) {
    let mut buffer = mmap_buffer(bt);
    let (_mmap_key, mmap_iter) = die_if_failure(bt.memory_map(&mut buffer));
    assert!(mmap_iter.len() > 0, "Memory map is empty");

    let entries: Vec<MemoryDescriptor> = mmap_iter
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

fn mmap_buffer(bt: &BootServices) -> Vec<u8> {
    let map_sz = bt.memory_map_size();

    let buf_sz = map_sz + 8 * core::mem::size_of::<MemoryDescriptor>();

    vec![0_u8; buf_sz]
}

fn die_if_failure<T, E>(res: uefi::Result<T, E>) -> T
where
    E: Debug,
{
    match res {
        Ok(v) => v.log(),
        Err(_) => loop {
            unsafe { asm!("nop") }
        },
    }
}
