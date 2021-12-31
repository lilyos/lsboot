#![no_main]
#![no_std]
#![feature(abi_efiapi, type_name_of_val, asm_const, asm_sym)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;

use log::info;

use uefi::{
    prelude::*,
    proto::media::{
        file::{File, FileAttribute, FileMode, FileType},
        fs::SimpleFileSystem,
    },
    table::boot::{MemoryDescriptor, MemoryType},
    ResultExt,
};

use core::arch::asm;

// mod utility;
// use utility::*;

#[macro_use]
mod elf;

#[no_mangle]
pub extern "efiapi" fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).expect_success("FAILED: BOOT SERVICES UNRESPONSIVE");

    info!("Loading Lotus");

    let prop = st
        .boot_services()
        .locate_protocol::<SimpleFileSystem>()
        .expect_success("FAILED: FILESYSTEM NOT FOUND");

    let prop = unsafe { &mut *prop.get() };

    let k_buf = load_file(prop, "EFI\\lotus\\bud");
    let t_buf = load_file(prop, "EFI\\lotus\\echo_sysv");

    let (kernel_stack, kernel_entry) = elf::load_elf!(k_buf, 4, st);
    let (_, test_entry) = elf::load_elf!(t_buf, 0, st);

    // info!("Test entry: {:?}", test_entry);

    let chk = u64::MAX;

    let test_fn: extern "sysv64" fn(u64) -> u64 = unsafe { core::mem::transmute(test_entry) };
    let res = test_fn(chk);

    if res == chk {
        info!("PASSED TEST: RES == CHK - {}", res);
        info!(
            "Jumping to Lotus entry {:?}, Stack at {:?}",
            kernel_entry, kernel_stack
        );

        info!("EXITING BOOT SERVICES");

        let map_sz = st.boot_services().memory_map_size();

        let buf_sz = map_sz + 2 * core::mem::size_of::<MemoryDescriptor>();

        let alloc_ptr = st
            .boot_services()
            .allocate_pool(MemoryType::LOADER_DATA, buf_sz)
            .expect_success("FAILED: COULDN'T ALLOCATE MEMORY");

        let mut buffer = unsafe { core::slice::from_raw_parts_mut(alloc_ptr as *mut u8, buf_sz) };

        let (_st, mmap_iter) = st
            .exit_boot_services(image, &mut buffer)
            .expect_success("FAILED: BOOT SERVICES NOT EXITED");

        let mmap = unsafe {
            core::slice::from_raw_parts_mut(alloc_ptr as *mut MemoryDescriptor, mmap_iter.len())
        };
        for (i, entry) in mmap_iter.enumerate() {
            mmap[i] = *entry;
        }

        unsafe {
            asm!(
                "cli",
                // "mov r8, {0}",
                // "mov r9, {1}",
                "mov rsp, {}",
                "jmp r8",
                in(reg) kernel_stack,
                in("r8") kernel_entry,
                in("r9") mmap.as_ptr() as *const MemoryDescriptor,
                in("r10") mmap.len(),
                options(noreturn)
            )
        }
    } else {
        panic!("FAILED: SELF TEST DIDN'T RETURN EXPECTED RESULTS");
    }
}

fn load_file(prop: &mut SimpleFileSystem, path: &str) -> Vec<u8> {
    let mut root = prop
        .open_volume()
        .expect_success("FAILED: ROOT DIRECTORY NOT ACCESSED");

    let kernel = root
        .open(path, FileMode::Read, FileAttribute::empty())
        .expect_success("FAILED: BUD NOT FOUND")
        .into_type()
        .expect_success("FAILED: BUD NOT A FILE");

    let mut kernel = match kernel {
        FileType::Regular(file) => file,
        _ => panic!("FAILED: BUD NOT A FILE"),
    };

    let mut println_buf = vec![0u8; 128];
    let kernel_size;
    loop {
        match kernel.get_info(&mut println_buf) {
            Ok(v) => {
                let v: &mut uefi::proto::media::file::FileInfo = v.log();
                kernel_size = v.file_size();
                break;
            }
            Err(e) => println_buf = vec![0u8; e.data().unwrap()],
        }
    }

    let mut kernel_buf = vec![0u8; kernel_size.try_into().unwrap()];
    kernel
        .read(&mut kernel_buf)
        .expect_success("FAILED: BUD UNREADABLE");

    kernel_buf
}
