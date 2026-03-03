#![no_std]
#![feature(abi_x86_interrupt)]

extern crate self as kernel;

use core::arch::asm;

pub mod gdt;
pub mod interrupts;

pub fn init() {
    qemu_debugcon::init_logger();

    gdt::init();
    interrupts::init_idt();
}

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") }
    }
}
