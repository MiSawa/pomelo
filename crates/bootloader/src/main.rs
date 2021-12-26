#![no_main]
#![no_std]
#![feature(abi_efiapi)]

use anyhow::{anyhow, bail, Context as _, Error, Result};
use core::fmt::Write;
use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileMode, FileType, RegularFile},
    ResultExt,
};

#[entry]
fn main(handle: Handle, st: SystemTable<Boot>) -> Status {
    actual_main(handle, st).expect("Failed!");
    Status::SUCCESS
}

fn actual_main(handle: Handle, mut st: SystemTable<Boot>) -> Result<()> {
    // Initialize utilities
    uefi_services::init(&mut st).expect_success("Failed to initialize utilities");
    st.stdout()
        .reset(false)
        .expect_success("Failed to reset stdout");

    writeln!(st.stdout(), "Hello, world!!!!").expect("Failed to write to stdout");

    let bs = st.boot_services();

    let root = open_root_dir(handle, bs)
        .warning_as_error()
        .map_err(|_| anyhow!("Failed to open a file to write the memory mapping"))?;
    write_memory_map_file(bs, root, "\\memmap")?;

    writeln!(st.stdout(), "Bye, world!!!!").expect("Failed to write to stdout");
    #[allow(clippy::empty_loop)]
    loop {}
    // Status::SUCCESS
}

fn open_root_dir(handle: Handle, bs: &BootServices) -> uefi::Result<Directory> {
    let fs = bs.get_image_file_system(handle).warning_as_error()?;
    let fs = unsafe { &mut *fs.interface.get() };
    fs.open_volume()
}

fn write_memory_map_file(bs: &BootServices, mut root: Directory, filename: &str) -> Result<()> {
    let mut memory_map = [0; 16 * 1024];
    let (_map_key, desc_iter) = bs
        .memory_map(&mut memory_map)
        .warning_as_error()
        .map_err(|_| anyhow!("Failed to get memory mapping"))?;

    let memory_map_file = root
        .open(filename, FileMode::CreateReadWrite, FileAttribute::empty())
        .warning_as_error()
        .map_err(|_| anyhow!("Failed to open a file to write the memory mapping"))?;
    struct FileWrapper(RegularFile);
    impl Write for FileWrapper {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0
                .write(s.as_bytes())
                .warning_as_error()
                .map_err(|_| core::fmt::Error)
        }
    }
    let mut file = match memory_map_file
        .into_type()
        .expect_success("Failed to get type of a file")
    {
        FileType::Regular(f) => FileWrapper(f),
        _ => bail!("memmap file exists as non-regular-file"),
    };

    writeln!(
        file,
        "Index, TYpe, Type(name), PhysicalStart, NumberOfPages, Attribute"
    )
    .map_err(Error::msg)
    .with_context(|| "Failed to write to the memory mapping file")?;
    for (i, desc) in desc_iter.enumerate() {
        writeln!(
            file,
            "{}, {:x}, {:?}, {:08x}, {}, {:x}",
            i, desc.ty.0, desc.ty, desc.phys_start, desc.page_count, desc.att
        )
        .map_err(Error::msg)
        .with_context(|| "Failed to write to the memory mapping file")?;
    }
    Ok(())
}
