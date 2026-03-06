#![no_std]

extern crate self as bootloader;

use bootloader::memory::EarlyFrameAllocator;
use bootloader::paging::PageTables;
use core::alloc::Layout;
use core::arch::asm;
use core::mem::MaybeUninit;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::PhysFrame;

pub mod kernel;
pub mod mappings;
pub mod memory;
pub mod paging;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_STACK_SIZE: u64 = 64 * 1024;
pub const KERNEL_STACK_TOP: VirtAddr = VirtAddr::new(0xFFFF_8180_0000_0000);
pub const BOOT_INFO_ADDR: VirtAddr = VirtAddr::new(0xFFFF_8180_2000_0000);

#[derive(Debug)]
pub struct BootInfo {}

pub fn create_boot_info<'a>(
    frame_allocator: &'a mut EarlyFrameAllocator,
    page_tables: &'a mut PageTables,
) -> &'static mut BootInfo {
    // Map boot info
    let layout = Layout::new::<BootInfo>();

    let boot_info_pages = Page::range_inclusive(
        Page::containing_address(BOOT_INFO_ADDR),
        Page::containing_address((BOOT_INFO_ADDR + layout.size() as u64) - 1u64),
    );

    for page in boot_info_pages {
        let frame = frame_allocator.allocate_frame().unwrap();
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        unsafe {
            page_tables
                .kernel_pml4_table
                .map_to(page, frame, flags, frame_allocator)
                .unwrap()
                .flush();
        };

        unsafe {
            page_tables
                .bootloader_pml4_table
                .map_to(page, frame, flags, frame_allocator)
                .unwrap()
                .flush();
        }
    }

    let boot_info: &'static mut MaybeUninit<BootInfo> =
        unsafe { &mut *BOOT_INFO_ADDR.as_mut_ptr() };

    let boot_info = boot_info.write(BootInfo {});

    boot_info
}

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") }
    }
}

pub unsafe fn context_switch(
    kernel_pml4_frame: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
    boot_info: &BootInfo,
) -> ! {
    unsafe {
        asm!(
            "xor rbp, rbp",
            "mov cr3, {}",
            "mov rsp, {}",
            "push 0",
            "jmp {}",
            in(reg) kernel_pml4_frame.start_address().as_u64(),
            in(reg) stack_top.as_u64(),
            in(reg) entry_point.as_u64(),
            in("rdi") boot_info as *const _ as usize,
            options(noreturn, nomem, preserves_flags)
        )
    };
}

#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        const _: () = {
            #[unsafe(no_mangle)]
            pub extern "C" fn _start(boot_info: &'static mut $crate::BootInfo) -> ! {
                let f: fn(&'static mut $crate::BootInfo) -> ! = $path;

                f(boot_info)
            }
        };
    };
}
