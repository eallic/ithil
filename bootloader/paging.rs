use bootloader::memory::EarlyFrameAllocator;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::registers::control::Cr3Flags;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PhysFrame;

pub struct PageTables {
    pub bootloader_pml4_table: OffsetPageTable<'static>,
    pub kernel_pml4_table: OffsetPageTable<'static>,
    pub kernel_pml4_frame: PhysFrame,
}

impl PageTables {
    pub fn new(frame_allocator: &mut EarlyFrameAllocator) -> Self {
        let current_frame = Cr3::read().0;
        let old_table_ptr: *const PageTable =
            VirtAddr::new(current_frame.start_address().as_u64()).as_ptr();
        let old_table = unsafe { &*old_table_ptr };

        let new_frame = frame_allocator.allocate_frame().unwrap();
        let new_table_ptr = VirtAddr::new(new_frame.start_address().as_u64()).as_mut_ptr();
        unsafe {
            *new_table_ptr = PageTable::new();
        }
        let new_table = unsafe { &mut *new_table_ptr };

        let end_addr = VirtAddr::new(frame_allocator.max_phys_addr().as_u64() - 1);
        for p4 in 0..=usize::from(end_addr.p4_index()) {
            new_table[p4] = old_table[p4].clone();
        }

        unsafe {
            Cr3::write(new_frame, Cr3Flags::empty());
        }
        let bootloader_pml4_table =
            unsafe { OffsetPageTable::new(&mut *new_table, VirtAddr::new(0)) };

        // Create a new page table for the kernel
        let kernel_pml4_frame = frame_allocator.allocate_frame().unwrap();
        let kernel_pml4_table_ptr =
            VirtAddr::new(kernel_pml4_frame.start_address().as_u64()).as_mut_ptr();
        unsafe {
            *kernel_pml4_table_ptr = PageTable::new();
        }
        let kernel_pml4_table =
            unsafe { OffsetPageTable::new(&mut *kernel_pml4_table_ptr, VirtAddr::new(0)) };

        Self {
            bootloader_pml4_table,
            kernel_pml4_table,
            kernel_pml4_frame,
        }
    }
}
