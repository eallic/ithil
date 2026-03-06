#![no_std]
#![no_main]

use bootloader::BootInfo;
use bootloader::entry_point;
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init();
    log::info!("Hello from the kernel");
    log::info!("{:#x?}", boot_info);

    kernel::hcf();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::info!("{info}");
    kernel::hcf();
}
