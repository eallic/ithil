#![no_std]
#![no_main]

use bootloader::PAGE_SIZE;
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
use xmas_elf::ElfFile;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    log::info!("Loading kernel");
    let kernel_bytes = load_file(cstr16!("KERNEL.ELF"));
    let kernel = ElfFile::new(kernel_bytes).unwrap();

    log::info!("Exiting boot services");
    let mut memory_map = unsafe { boot::exit_boot_services(None) };
    memory_map.sort();

    let mut frame_allocator = EarlyFrameAllocator::new(memory_map);
    let mut page_tables = PageTables::new(&mut frame_allocator);

    let mappings = Mappings::new(
        &kernel,
        &mut frame_allocator,
        &mut page_tables.kernel_pml4_table,
    );

    unsafe {
        bootloader::context_switch(
            page_tables.kernel_pml4_frame,
            mappings.stack_top,
            mappings.entry_point,
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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");

    bootloader::hcf();
}
