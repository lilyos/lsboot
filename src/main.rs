#![no_main]
#![no_std]
#![feature(abi_efiapi, type_name_of_val, asm_const)]

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
    ResultExt,
};

use core::arch::asm;

mod utility;
use utility::*;

#[macro_use]
mod efi;
use efi::*;

#[entry]
pub fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).expect_success("FAILED: BOOT SERVICES UNRESPONSIVE");

    info!("Loading Lotus");

    get_memory_map(st.boot_services());

    let prop = st
        .boot_services()
        .locate_protocol::<SimpleFileSystem>()
        .expect_success("FAILED: FILESYSTEM NOT FOUND");

    let prop = unsafe { &mut *prop.get() };

    let kernel_buf = load_file(prop, "EFI\\lotus\\bud");
    let test_buf = load_file(prop, "EFI\\lotus\\echo_sysv");

    let (kernel_stack, kernel_entry) = efi::load_efi!(kernel_buf, st);
    let (_test_stack, test_entry) = efi::load_efi!(test_buf, st);

    let chk = 12341234;

    let test_fn: extern "sysv64" fn(u64) -> u64 = unsafe { core::mem::transmute(test_entry) };
    let res = test_fn(chk);

    let mut owo = 11u64;
    let ptr = &mut owo as *mut u64;
    if res == chk {
        info!("PASSED TEST: RES == CHK - {}", res);
        info!(
            "Jumping to Lotus entry {:?}, Flag at {:?}, Stack at {:?}",
            kernel_entry, ptr, kernel_stack
        );
        unsafe { asm!("inc qword ptr [{}]", in(reg) ptr) } // 12
        exit_boot_services(st, image);
        unsafe {
            asm!("inc qword ptr [{}]", in(reg) ptr); // 13? No
            asm!("cli");
            asm!("mov rdi, {}", in(reg) ptr);
            asm!("mov rsp, {}", in(reg) kernel_stack);
            asm!("jmp {}", in(reg) kernel_entry, options(noreturn));
        }
    } else {
        panic!("SELF TEST FAILED");
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
        _ => panic!("Bud not a file"),
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
        .expect_success("FAILED: COULDN'T READ BUD");

    kernel_buf
}
