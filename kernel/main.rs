#![no_std]
#![no_main]

use bootloader::entry_point;
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main() -> ! {
    qemu_debugcon::init_logger();
    log::info!("Hello from the kernel");

    kernel::hcf();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::info!("{info}");
    kernel::hcf();
}
