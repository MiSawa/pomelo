#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![feature(int_roundings)]

#[macro_use]
extern crate alloc;

use anyhow::{anyhow, bail, Context as _, Error, Result};
use core::{arch::asm, fmt::Write};
use object::{elf, read::elf::ProgramHeader as _, Endianness};
use pomelo_common::{GraphicConfig, KernelArg, PixelFormat};
use uefi::{
    prelude::*,
    proto::{
        console::gop::GraphicsOutput,
        media::file::{Directory, File, FileAttribute, FileInfo, FileMode, FileType, RegularFile},
    },
    table::boot::{AllocateType, MemoryType},
};

#[entry]
fn main(handle: Handle, st: SystemTable<Boot>) -> Status {
    actual_main(handle, st).expect("Failed!");
    Status::SUCCESS
}

fn actual_main(handle: Handle, mut st: SystemTable<Boot>) -> Result<()> {
    uefi_services::init(&mut st).expect_success("Failed to initialize utilities");
    st.stdout()
        .reset(false)
        .expect_success("Failed to reset stdout");

    writeln!(st.stdout(), "Hello, world!!!!").expect("Failed to write to stdout");

    let mut root = open_root_dir(handle, st.boot_services())
        .warning_as_error()
        .map_err(|_| anyhow!("Failed to open a file to write the memory mapping"))?;

    write_memory_map_file(st.boot_services(), &mut root, "\\memmap")?;
    writeln!(st.stdout(), "Wrote memory map file").expect("Failed to write to stdout");

    let kernel_main =
        prepare_kernel::<elf::FileHeader64<Endianness>>(st.boot_services(), &mut root, "\\kernel")?;
    writeln!(st.stdout(), "Loaded kernel").expect("Failed to write to stdout");

    let graphic_config = read_graphic_config(&mut st)?;

    let mut memory_map = [0; 16 * 1024];
    let (_st, _) = st
        .exit_boot_services(handle, &mut memory_map)
        .expect_success("Failed to exit boot services");

    kernel_main(KernelArg { graphic_config });

    #[allow(clippy::empty_loop)]
    loop {
        unsafe { asm!("hlt") }
    }
}

fn open_root_dir(handle: Handle, bs: &BootServices) -> uefi::Result<Directory> {
    let fs = bs.get_image_file_system(handle).warning_as_error()?;
    let fs = unsafe { &mut *fs.interface.get() };
    fs.open_volume()
}

fn read_graphic_config(st: &mut SystemTable<Boot>) -> Result<GraphicConfig> {
    let go = st
        .boot_services()
        .locate_protocol::<GraphicsOutput>()
        .warning_as_error()
        .map_err(|_| anyhow!("Unable to get graphics output"))?;
    let go = unsafe { &mut *go.get() };

    let mode = go.modes().find_map(|mode| {
        let mode = mode.expect("Unable to get mode");
        let format = mode.info().pixel_format();
        match format {
            uefi::proto::console::gop::PixelFormat::Rgb => Option::Some((mode, PixelFormat::Rgb)),
            uefi::proto::console::gop::PixelFormat::Bgr => Option::Some((mode, PixelFormat::Bgr)),
            _ => Option::None,
        }
    });
    let (mode, pixel_format) =
        mode.ok_or_else(|| anyhow!("Unable to find supported pixel format (RGB | BGR)"))?;
    go.set_mode(&mode)
        .warning_as_error()
        .map_err(|_| anyhow!("Unable to set mode"))?;
    let info = mode.info();
    let (horisontal_resolution, vertical_resolution) = info.resolution();
    let stride = info.stride();

    let mut fb = go.frame_buffer();
    let config = GraphicConfig {
        frame_buffer_base: fb.as_mut_ptr(),
        frame_buffer_size: fb.size(),
        pixel_format,
        horisontal_resolution,
        vertical_resolution,
        stride,
    };
    Ok(config)
}

fn prepare_kernel<Elf: object::read::elf::FileHeader<Endian = Endianness>>(
    bs: &BootServices,
    root: &mut Directory,
    filename: &str,
) -> Result<extern "sysv64" fn(KernelArg)> {
    let kernel_file = root
        .open(filename, FileMode::Read, FileAttribute::empty())
        .warning_as_error()
        .map_err(|_| anyhow!("Failed to open the kernel file"))?;
    let mut kernel_file = match kernel_file
        .into_type()
        .expect_success("Failed to get type of a file")
    {
        FileType::Regular(f) => f,
        _ => bail!("kernel file exists as non-regular-file"),
    };
    let mut file_info_buffer = [0; 8192];
    let kernel_file_info = kernel_file
        .get_info::<FileInfo>(&mut file_info_buffer)
        .expect_success("Failed to get file info");
    let kernel_file_size = kernel_file_info.file_size() as usize;
    let mut kernel_content = vec![0; kernel_file_size];
    kernel_file
        .read(kernel_content.as_mut_slice())
        .expect_success("Unable to read kernel file content");

    let elf = Elf::parse(kernel_content.as_slice())
        .map_err(|_| anyhow!("Unable to parse the kernel file as elf"))?;
    let endian = elf
        .endian()
        .map_err(|_| anyhow!("Unable to determin endian of the kernel file"))?;

    let entry_point = elf.e_entry(endian).into() as usize;
    let (kernel_base_address, kernel_length) = {
        let mut start = u64::MAX;
        let mut end = u64::MIN;

        for segment in elf
            .program_headers(endian, kernel_content.as_slice())
            .map_err(|_| anyhow!("Unable to parse program headers of the kernel"))?
        {
            if segment.p_type(endian) == elf::PT_LOAD {
                let start_pos = segment.p_vaddr(endian).into();
                let end_pos = start_pos + segment.p_memsz(endian).into();
                start = start.min(start_pos);
                end = end.max(end_pos);
            }
        }
        (start as usize, (end - start) as usize)
    };
    const PAGE_SIZE: usize = 0x1000;
    let allocate_page_count = kernel_length.div_ceil(PAGE_SIZE);
    bs.allocate_pages(
        AllocateType::Address(kernel_base_address),
        MemoryType::LOADER_DATA,
        allocate_page_count,
    )
    .expect_success("Failed to allocate pages");
    let allocated_slice = unsafe {
        core::slice::from_raw_parts_mut(
            kernel_base_address as *mut u8,
            allocate_page_count * PAGE_SIZE,
        )
    };
    for segment in elf
        .program_headers(endian, kernel_content.as_slice())
        .map_err(|_| anyhow!("Unable to parse program headers of the kernel"))?
    {
        if segment.p_type(endian) == elf::PT_LOAD {
            let start_pos = segment.p_vaddr(endian).into() as usize - kernel_base_address;
            let end_pos = start_pos + segment.p_memsz(endian).into() as usize;
            let data = segment
                .data(endian, kernel_content.as_slice())
                .map_err(|_| anyhow!("Unable to read segment from kernel"))?;
            let copy_from_file_end_pos = start_pos + data.len();
            allocated_slice[start_pos..copy_from_file_end_pos].copy_from_slice(data);
            allocated_slice[copy_from_file_end_pos..end_pos].fill(0);
        }
    }
    kernel_file
        .read(allocated_slice)
        .expect_success("Failed to read kernel into allocated memory");

    let entry_point: extern "sysv64" fn(KernelArg) = unsafe { core::mem::transmute(entry_point) };
    Ok(entry_point)
}

fn write_memory_map_file(bs: &BootServices, root: &mut Directory, filename: &str) -> Result<()> {
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
