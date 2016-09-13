/* esh - embedded shell
 *
 * Copyright (c) 2016, Chris Pavlina.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
 * OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
 * OR OTHER DEALINGS IN THE SOFTWARE.
 */

use std::ptr;
use std::mem;
use std::slice;
use std::ops::Index;

/// The main esh object. This is an opaque object representing an esh instance,
/// and having methods for interacting with it.
pub enum Esh {}
enum Void {}

/// Argument accessor. Provides a bound-checked interface to argc/argv without
/// requiring any allocation.
pub struct EshArgArray {
    argc: i32,
    argv: *mut *mut u8,
}

extern "C" {
    fn esh_init() -> *mut Esh;
    fn esh_register_command(
        esh: *mut Esh,
        cb: extern fn(esh: *mut Esh, argc: i32, argv: *mut *mut u8, arg: *mut Void),
        arg: *mut Void);
    fn esh_register_print(
        esh: *mut Esh,
        cb: extern "C" fn(esh: *mut Esh, s: *const u8, arg: *mut Void),
        arg: *mut Void);
    fn esh_register_overflow(
        esh: *mut Esh,
        cb: extern "C" fn(*mut Esh, *const u8, *mut Void),
        arg: *mut Void);
    fn esh_rx(esh: *mut Esh, c: u8);
}

fn strlen(s: *const u8) -> usize {
    let mut i: usize = 0;
    loop {
        let c = unsafe{*s.offset(i as isize)};
        if c == 0 {
            break;
        } else {
            i += 1;
        }
    }
    return i;
}

impl Esh {
    /// Return an initialized esh object. Must be called before any other
    /// functions.
    ///
    /// See `ESH_ALLOC` in `esh_config.h` - this should be `STATIC` or
    /// `MALLOC`. If `STATIC`, `ESH_INSTANCES` must be defined to the
    /// maximum number of instances, and references to these instances
    /// will be returned.
    ///
    /// Return value:
    ///
    /// * `Some(&'static mut Esh)` - successful initialization
    /// * `None` - static instance count was exceeded or malloc failed.
    pub fn init() -> Option<&'static mut Esh> {
        let esh = unsafe{esh_init()};
        if esh == ptr::null_mut() {
            return None;
        } else {
            return Some(unsafe{&mut *esh});
        }
    }

    extern "C" fn print_callback_wrapper(esh: *mut Esh, s: *const u8, arg: *mut Void) {
        let func: fn(&Esh, &[u8]) = unsafe{mem::transmute(arg)};

        let i = strlen(s);
        let string_slice = unsafe{slice::from_raw_parts(s, i)};
        let esh_self = unsafe{&*esh};

        func(esh_self, string_slice);
    }

    /// Register a callback to print a string.
    ///
    /// Callback arguments:
    ///
    /// * `esh` - the originating esh instance, allowing identification
    /// * `s` - the string to print, as a slice of bytes
    pub fn register_print(&mut self, cb: fn(esh: &Esh, s: &[u8])) {
        let fp = cb as *mut Void;
        unsafe {
            esh_register_print(self, Esh::print_callback_wrapper, fp);
        }
    }

    extern "C" fn command_callback_wrapper(
            esh: *mut Esh, argc: i32, argv: *mut *mut u8, arg: *mut Void) {

        let func: fn(&Esh, &EshArgArray) = unsafe{mem::transmute(arg)};
        let argarray = EshArgArray{argc: argc, argv: argv};
        let esh_self = unsafe{&*esh};
        func(esh_self, &argarray);
    }

    /// Register a callback to execute a command.
    ///
    /// Callback arguments:
    ///
    /// * `esh` - the originating esh instance, allowing identification
    /// * `args` - a reference to an argument accessor object
    pub fn register_command(&mut self, cb: fn(esh: &Esh, args: &EshArgArray)) {
        let fp = cb as *mut Void;
        unsafe {
            esh_register_command(self, Esh::command_callback_wrapper, fp);
        }
    }

    extern "C" fn overflow_callback_wrapper(
            esh: *mut Esh, buf: *const u8, arg: *mut Void) {

        let func: fn(&Esh, &[u8]) = unsafe{mem::transmute(arg)};
        let i = strlen(buf);
        let buf_slice = unsafe{slice::from_raw_parts(buf, i)};
        let esh_self = unsafe{&*esh};

        func(esh_self, buf_slice);
    }

    /// Register a callback to notify about overflow. Optional; esh has an
    /// internal overflow handler.
    ///
    /// Callback arguments:
    ///
    /// * `esh` - the originating esh instance, allowing identification
    /// * `s` - the contents of the buffer, excluding overflow
    pub fn register_overflow(&mut self, cb: fn(esh: &Esh, s: &[u8])) {
        let fp = cb as *mut Void;
        unsafe {
            esh_register_overflow(self, Esh::overflow_callback_wrapper, fp);
        }
    }

    /// Pass in a character that was received.
    pub fn rx(&mut self, c: u8) {
        unsafe {
            esh_rx(self, c);
        }
    }
}

impl EshArgArray {

    /// Return the number of arguments, including the command name.
    pub fn len(&self) -> usize {
        return self.argc as usize;
    }
}

impl Index<usize> for EshArgArray {
    type Output = [u8];

    /// Return an argument. Indices start from zero, with args[0] being the
    /// command name. If `index` is out of bounds, an empty argument is
    /// returned.
    fn index<'a> (&'a self, index: usize) -> &'a [u8] {
        if index >= self.argc as usize {
            return &[];
        } else {
            let arg = unsafe{*self.argv.offset(index as isize)};
            let len = strlen(arg);
            return unsafe{slice::from_raw_parts(arg, len)};
        }
    }
}

