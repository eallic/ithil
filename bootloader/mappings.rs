use bootloader::KERNEL_STACK_SIZE;
use bootloader::KERNEL_STACK_TOP;
use bootloader::kernel;
use bootloader::memory::EarlyFrameAllocator;
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::PhysFrame;
use xmas_elf::ElfFile;

pub struct Mappings {
    pub stack_top: VirtAddr,
    pub entry_point: VirtAddr,
}

impl Mappings {
    pub fn new<'a>(
        kernel: &ElfFile,
        frame_allocator: &'a mut EarlyFrameAllocator,
        kernel_pml4_table: &'a mut OffsetPageTable<'static>,
    ) -> Self {
        let entry_point = kernel::load_kernel(kernel, kernel_pml4_table, frame_allocator);

        // Map stack
        let stack_pages = Page::range_inclusive(
            Page::containing_address(KERNEL_STACK_TOP - KERNEL_STACK_SIZE),
            Page::containing_address(KERNEL_STACK_TOP - 1),
        );

        for page in stack_pages {
            let frame = frame_allocator.allocate_frame().unwrap();
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe {
                kernel_pml4_table
                    .map_to(page, frame, flags, frame_allocator)
                    .unwrap()
                    .flush();
            };
        }

        // Map context switch function
        let context_switch_fn = PhysAddr::new(bootloader::context_switch as *const () as u64);
        let context_switch_fn_start_frame: PhysFrame =
            PhysFrame::containing_address(context_switch_fn);

        let context_switch_frames = PhysFrame::range_inclusive(
            context_switch_fn_start_frame,
            context_switch_fn_start_frame + 1,
        );

        for frame in context_switch_frames {
            let page = Page::containing_address(VirtAddr::new(frame.start_address().as_u64()));
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe {
                kernel_pml4_table
                    .map_to(page, frame, flags, frame_allocator)
                    .unwrap()
                    .flush();
            };
        }

        Self {
            stack_top: stack_pages.end.start_address().align_down(16u8),
            entry_point,
        }
    }
}
