use bootloader::KERNEL_STACK_SIZE;
use bootloader::kernel;
use bootloader::memory::EarlyFrameAllocator;
use bootloader::paging::PageTables;
use bootloader::paging::UsedLevel4Entries;
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;
use xmas_elf::ElfFile;

pub struct Mappings {
    pub stack_top: VirtAddr,
    pub entry_point: VirtAddr,
}

impl Mappings {
    pub fn new<'a>(
        kernel: &ElfFile,
        frame_allocator: &'a mut EarlyFrameAllocator,
        page_tables: &'a mut PageTables,
    ) -> Self {
        let mut kernel_pml4_table = &mut page_tables.kernel_pml4_table;

        let mut used_entries =
            UsedLevel4Entries::new(frame_allocator.max_phys_addr(), &kernel).unwrap();

        let entry_point = kernel::load_kernel(
            kernel,
            &mut kernel_pml4_table,
            frame_allocator,
            &mut used_entries,
        );

        let stack_start = {
            let addr = used_entries.get_free_address(Size4KiB::SIZE + KERNEL_STACK_SIZE);
            let guard_page = Page::from_start_address(addr).unwrap();
            guard_page + 1
        };
        let stack_end_addr = stack_start.start_address() + KERNEL_STACK_SIZE;

        let stack_end = Page::containing_address(stack_end_addr - 1u64);
        for page in Page::range_inclusive(stack_start, stack_end) {
            let frame = frame_allocator.allocate_frame().unwrap();
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            let flusher = unsafe {
                kernel_pml4_table
                    .map_to(page, frame, flags, frame_allocator)
                    .unwrap()
            };
            flusher.flush();
        }

        let context_switch_fn = PhysAddr::new(bootloader::context_switch as *const () as u64);
        let context_switch_fn_start_frame: PhysFrame =
            PhysFrame::containing_address(context_switch_fn);
        for frame in PhysFrame::range_inclusive(
            context_switch_fn_start_frame,
            context_switch_fn_start_frame + 1,
        ) {
            let page = Page::containing_address(VirtAddr::new(frame.start_address().as_u64()));
            let flusher = unsafe {
                kernel_pml4_table
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        frame_allocator,
                    )
                    .unwrap()
            };

            flusher.flush()
        }

        Self {
            stack_top: stack_end_addr.align_down(16u8),
            entry_point,
        }
    }
}
