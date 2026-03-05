#![no_std]

extern crate self as bootloader;

use core::arch::asm;

use x86_64::VirtAddr;
use x86_64::structures::paging::PhysFrame;

pub mod kernel;
pub mod mappings;
pub mod memory;
pub mod paging;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_STACK_SIZE: u64 = 64 * 1024;
pub const KERNEL_STACK_TOP: VirtAddr = VirtAddr::new(0xFFFF_8180_0000_0000);

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") }
    }
}

pub unsafe fn context_switch(
    kernel_pml4_frame: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
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
            options(noreturn, nomem, preserves_flags)
        )
    };
}
