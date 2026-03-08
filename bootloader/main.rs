#![no_std]
#![no_main]

use bootloader::FRAMEBUFFER_ADDR;
use bootloader::Framebuffer;
use bootloader::PAGE_SIZE;
use bootloader::PixelFormat;
use bootloader::mappings::Mappings;
use bootloader::memory::EarlyFrameAllocator;
use bootloader::paging::PageTables;
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
use uefi::table::cfg::ConfigTableEntry;
use x86_64::PhysAddr;
use xmas_elf::ElfFile;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    log::info!("Loading kernel");
    let kernel_bytes = load_file(cstr16!("KERNEL.ELF"));
    let kernel = ElfFile::new(kernel_bytes).unwrap();

    log::info!("Loading framebuffer");
    let framebuffer = load_framebuffer();
    log::info!("Loaded framebuffer: {:#?}", framebuffer);

    log::info!("Exiting boot services");
    let mut memory_map = unsafe { boot::exit_boot_services(None) };
    memory_map.sort();

    let mut frame_allocator = EarlyFrameAllocator::new(memory_map);
    let mut page_tables = PageTables::new(&mut frame_allocator, framebuffer);

    let mappings = Mappings::new(
        &kernel,
        &mut frame_allocator,
        &mut page_tables.kernel_pml4_table,
        framebuffer,
    );

    let rsdp_addr = uefi::system::with_config_table(|entries| {
        let acpi2_rsdp = entries
            .iter()
            .find(|entry| entry.guid == ConfigTableEntry::ACPI2_GUID);

        let acpi1_rsdp = entries
            .iter()
            .find(|entry| entry.guid == ConfigTableEntry::ACPI_GUID);

        let rsdp = acpi2_rsdp.or_else(|| acpi1_rsdp);

        rsdp.map(|entry| PhysAddr::new(entry.address as u64))
    });

    let boot_info = bootloader::create_boot_info(
        &mut frame_allocator,
        &mut page_tables,
        rsdp_addr,
        framebuffer,
    );

    unsafe {
        bootloader::context_switch(
            page_tables.kernel_pml4_frame,
            mappings.stack_top,
            mappings.entry_point,
            boot_info,
        );
    }
}

fn load_file(path: &CStr16) -> &'static mut [u8] {
    let sfs = boot::get_image_file_system(boot::image_handle()).unwrap();
    let mut fs = FileSystem::new(sfs);

    let file_info = fs.metadata(path).unwrap();
    let file_size = file_info.file_size() as usize;

    let file_ptr = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        file_size.div_ceil(PAGE_SIZE),
    )
    .unwrap()
    .as_ptr();

    let src = fs.read(path).unwrap();
    let dst = unsafe { slice::from_raw_parts_mut(file_ptr, file_size) };
    dst.copy_from_slice(&src);

    dst
}

fn load_framebuffer() -> Option<Framebuffer> {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().ok()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).ok()?;

    let mode_info = gop.current_mode_info();
    let mut gop_framebuffer = gop.frame_buffer();

    Some(Framebuffer {
        virt_addr: FRAMEBUFFER_ADDR,
        phys_addr: PhysAddr::new(gop_framebuffer.as_mut_ptr() as u64),
        byte_len: gop.frame_buffer().size(),
        width: mode_info.resolution().0,
        height: mode_info.resolution().1,
        stride: mode_info.stride(),
        bpp: 4,
        pixel_format: match mode_info.pixel_format() {
            gop::PixelFormat::Rgb => PixelFormat::Rgb,
            gop::PixelFormat::Bgr => PixelFormat::Bgr,
            _ => panic!("Unsupported pixel format"),
        },
    })
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");

    bootloader::hcf();
}
