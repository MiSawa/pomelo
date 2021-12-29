/// This file is copied and then modified from https://github.com/gifnksm/sabios/blob/a0729dbdaafbbc318c6bc13636a3a17a842c782b/src/cxx_support.rs
/// which is distributed under the following license.
///
/// MIT License
/// Copyright (c) 2021 gifnksm <makoto.nksm+github@gmail.com>
///
/// Permission is hereby granted, free of charge, to any person obtaining a copy
/// of this software and associated documentation files (the "Software"), to deal
/// in the Software without restriction, including without limitation the rights
/// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// copies of the Software, and to permit persons to whom the Software is
/// furnished to do so, subject to the following conditions:
///
/// The above copyright notice and this permission notice shall be included in all
/// copies or substantial portions of the Software.
///
/// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// SOFTWARE.

use crate::log::{self, Level};
use core::{ptr, slice, str};

#[no_mangle]
extern "C" fn sabios_log(
    level: i32,
    file: *const u8,
    file_len: usize,
    line: u32,
    msg: *const u8,
    msg_len: usize,
    cont_line: bool,
) -> i32 {
    let level = match level {
        3 => Level::Error,
        4 => Level::Warn,
        7 => Level::Debug,
        8 => Level::Trace,
        _ => Level::Info,
    };

    unsafe {
        let msg = slice::from_raw_parts(msg, msg_len);
        let msg = str::from_utf8_unchecked(msg);
        let file = slice::from_raw_parts(file, file_len);
        let file = str::from_utf8_unchecked(file);
        let newline = msg.ends_with('\n');
        log::_log(
            level,
            format_args!("{}", msg.trim_end()),
            file,
            line,
            cont_line,
            newline,
        );
    }

    msg_len as i32
}

extern "C" {
    fn __errno() -> *mut i32;
}

#[allow(non_camel_case_types)]
type pid_t = i32;
const EBADF: i32 = 9;
const ENOMEM: i32 = 12;
const EINVAL: i32 = 22;

#[no_mangle]
extern "C" fn sbrk(_increment: isize) -> *const u8 {
    ptr::null()
}

#[no_mangle]
extern "C" fn _exit() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
extern "C" fn kill(_pid: pid_t, _sig: i32) -> i32 {
    unsafe {
        *__errno() = EINVAL;
    }
    -1
}

#[no_mangle]
extern "C" fn getpid() -> pid_t {
    unsafe {
        *__errno() = EINVAL;
    }
    -1
}

#[no_mangle]
extern "C" fn close() -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn read(_fd: i32, _buf: *mut u8, _count: usize) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn write(_fd: i32, _buf: *const u8, _count: usize) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn lseek(_fd: i32, _offset: isize, _whence: i32) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn fstat(_fd: i32, _buf: *mut u8) -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn isatty(_fd: i32) -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn posix_memalign(_memptr: *mut *mut u8, _alignment: usize, _size: usize) -> i32 {
    ENOMEM
}
