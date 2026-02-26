#![no_std]

extern crate self as kernel;

use core::arch::asm;

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") }
    }
}
