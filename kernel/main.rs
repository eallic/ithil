#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    kernel::hcf();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kernel::hcf();
}
