use bootloader::memory::EarlyFrameAllocator;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PhysFrame;

pub struct PageTables {
    pub kernel_pml4_table: &'static PageTable,
    pub kernel_pml4_frame: PhysFrame,
}

impl PageTables {
    pub fn new(frame_allocator: &mut EarlyFrameAllocator) -> Self {
        // Create a new page table for the kernel
        let kernel_pml4_frame = frame_allocator.allocate_frame().unwrap();
        let kernel_pml4_table_ptr =
            VirtAddr::new(kernel_pml4_frame.start_address().as_u64()).as_mut_ptr();
        unsafe { *kernel_pml4_table_ptr = PageTable::new() };
        let kernel_pml4_table = unsafe { &mut *kernel_pml4_table_ptr };

        Self {
            kernel_pml4_table,
            kernel_pml4_frame,
        }
    }
}
