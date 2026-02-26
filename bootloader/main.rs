#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use uefi::prelude::*;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    panic!("test");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");

    loop {
        unsafe { asm!("cli; hlt") }
    }
}
