#![no_std]
#![no_main]

use bootloader::Framebuffer;
use bootloader::PixelFormat;
use core::panic::PanicInfo;
use core::slice;
use uefi::CStr16;
use uefi::boot::AllocateType;
use uefi::boot::MemoryType;
use uefi::fs::FileSystem;
use uefi::mem::memory_map::MemoryMapMut;
use uefi::prelude::*;
use uefi::proto::console::gop;
use uefi::proto::console::gop::GraphicsOutput;
use x86_64::PhysAddr;
use xmas_elf::ElfFile;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    log::info!("Loading kernel");
    let kernel_bytes = load_file(cstr16!("KERNEL.ELF"));
    let _kernel = ElfFile::new(kernel_bytes).unwrap();

    log::info!("Loading framebuffer");
    let framebuffer = load_framebuffer();
    log::info!("Loaded framebuffer: {:#?}", framebuffer);

    log::info!("Exiting boot services");
    let mut memory_map = unsafe { boot::exit_boot_services(None) };
    memory_map.sort();

    bootloader::hcf();
}

fn load_file(path: &CStr16) -> &'static mut [u8] {
    let sfs = boot::get_image_file_system(boot::image_handle()).unwrap();
    let mut fs = FileSystem::new(sfs);

    let file_info = fs.metadata(path).unwrap();
    let file_size = file_info.file_size() as usize;

    let file_ptr = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        file_size.div_ceil(4096),
    )
    .unwrap()
    .as_ptr();

    let src = fs.read(path).unwrap();
    let dst = unsafe { slice::from_raw_parts_mut(file_ptr, file_size) };
    dst.copy_from_slice(&src);

    dst
}

fn load_framebuffer() -> Framebuffer {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let mode_info = gop.current_mode_info();
    let mut gop_framebuffer = gop.frame_buffer();

    Framebuffer {
        addr: PhysAddr::new(gop_framebuffer.as_mut_ptr() as u64),
        width: mode_info.resolution().0,
        height: mode_info.resolution().1,
        stride: mode_info.stride(),
        bpp: 4,
        pixel_format: match mode_info.pixel_format() {
            gop::PixelFormat::Rgb => PixelFormat::Rgb,
            gop::PixelFormat::Bgr => PixelFormat::Bgr,
            _ => panic!("Unsupported pixel format"),
        },
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");

    bootloader::hcf();
}
