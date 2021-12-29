#![no_main]
#![no_std]
#![feature(once_cell)]

#[macro_use]
extern crate lazy_static;

mod cxx_support;
pub mod graphics;
pub mod mouse;
pub mod pci;
pub mod x86_64;
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
    use ::core::fmt::Write;
    let mut writer = crate::graphics::console::global_console();
    writer.write_fmt(args).unwrap();
}

pub mod prelude {
    pub use crate::{
        graphics::{ICoordinate, Point, Size, UCoordinate, Vector2d},
        print, println,
    };

    pub type Result<T> = ::core::result::Result<T, Error>;
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub enum Error {}
}
