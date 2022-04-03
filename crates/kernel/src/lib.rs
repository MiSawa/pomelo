#![no_main]
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(never_type)]
#![feature(maybe_uninit_uninit_array)]
#![feature(int_roundings)]
#![feature(generic_const_exprs)]
#![feature(naked_functions)]

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
pub(crate) mod keyboard;
pub mod logger;
pub(crate) mod memory_manager;
pub mod mpsc;
pub mod msi;
pub mod paging;
pub mod pci;
pub(crate) mod ring_buffer;
pub mod task;
#[allow(unused)]
pub mod timer;
pub mod triple_buffer;
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
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut writer = crate::gui::widgets::console::global_console();
        // let mut writer = crate::gui::widgets::console::fallback_console();
        writer.write_fmt(args).ok();
    })
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
