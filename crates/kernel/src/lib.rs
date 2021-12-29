#![no_main]
#![no_std]
#![feature(once_cell)]

#[macro_use]
extern crate lazy_static;

pub mod graphics;
pub mod pci;
pub mod x86_64;
