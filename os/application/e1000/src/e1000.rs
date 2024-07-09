#![no_std]

extern crate alloc;

use io::{print, println};
//required for panic handler
use runtime::*;
#[no_mangle]
pub fn main() {
    println!("Hello, world!");
}
