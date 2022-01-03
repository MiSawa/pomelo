#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(never_type)]
#![feature(maybe_uninit_uninit_array)]
#![feature(int_roundings)]
#![feature(generic_const_exprs)]
#![feature(ptr_to_from_bits)]
#![feature(once_cell)]

#[macro_use]
extern crate lazy_static;
extern crate alloc;

pub mod allocator;
pub(crate) mod bitset;
mod cxx_support;
pub mod events;
pub mod gdt;
pub mod graphics;
pub mod gui;
pub mod interrupts;
pub mod logger;
pub(crate) mod memory_manager;
pub(crate) mod mouse;
pub mod msi;
pub mod paging;
pub mod pci;
pub(crate) mod ring_buffer;
#[allow(unused)]
pub mod timer;
pub mod xhci;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print_impl(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print_impl(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    let mut writer = crate::graphics::widgets::console::global_console();
    writer.write_fmt(args).unwrap();
}

pub mod prelude {
    pub use crate::{
        graphics::{ICoordinate, Point, Size, UCoordinate, Vector2d},
        print, println,
    };

    pub type Result<T> = ::core::result::Result<T, Error>;
    #[derive(Debug)]
    pub enum Error {
        LogInitializeError(log::SetLoggerError),
        AcpiError(acpi::AcpiError),
        Whatever(&'static str),
    }
    impl From<log::SetLoggerError> for Error {
        fn from(e: log::SetLoggerError) -> Self {
            Self::LogInitializeError(e)
        }
    }
    impl From<acpi::AcpiError> for Error {
        fn from(e: acpi::AcpiError) -> Self {
            Self::AcpiError(e)
        }
    }
}
