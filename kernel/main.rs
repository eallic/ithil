#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    kernel::init();
    log::info!("Hello from the kernel");

    kernel::hcf();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::info!("{info}");
    kernel::hcf();
}
