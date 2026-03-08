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

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        for pixel in framebuffer.buffer_mut().chunks_exact_mut(4) {
            pixel[0] = 255;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 0;
        }
    }

    kernel::hcf();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::info!("{info}");
    kernel::hcf();
}
