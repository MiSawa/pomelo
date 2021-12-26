#![no_main]
#![no_std]
#![feature(abi_efiapi)]

use core::fmt::Write;
use uefi::prelude::*;
use uefi::ResultExt;

#[entry]
fn main(_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize utilities
    uefi_services::init(&mut system_table).expect_success("Failed to initialize utilities");
    system_table
        .stdout()
        .reset(false)
        .expect_success("Failed to reset stdout");

    writeln!(system_table.stdout(), "Hello, world!!!!").expect("Failed to write to stdout");

    #[allow(clippy::empty_loop)]
    loop {}
    // Status::SUCCESS
}

