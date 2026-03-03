use bootloader::kernel::calc_memory_requirements;
use bootloader::memory::EarlyFrameAllocator;
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PageTableIndex;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;
use xmas_elf::ElfFile;
use xmas_elf::program::ProgramHeader;

pub struct PageTables {
    pub kernel_pml4_table: OffsetPageTable<'static>,
    pub kernel_pml4_frame: PhysFrame,
}

impl PageTables {
    pub fn new(frame_allocator: &mut EarlyFrameAllocator) -> Self {
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
            kernel_pml4_table,
            kernel_pml4_frame,
        }
    }
}

pub struct UsedLevel4Entries {
    entry_state: [bool; 512],
}

impl UsedLevel4Entries {
    pub fn new(max_phys_addr: PhysAddr, kernel: &ElfFile<'_>) -> Result<Self, &'static str> {
        let mut used = UsedLevel4Entries {
            entry_state: [false; 512],
        };

        used.mark_range_as_used(0, max_phys_addr.as_u64());

        let (size, min_addr) = calc_memory_requirements(kernel);
        used.mark_range_as_used(min_addr, size);

        Ok(used)
    }

    fn mark_range_as_used(&mut self, address: u64, size: u64) {
        let start = VirtAddr::new(address);
        let end_inclusive = start + (size - 1);
        let start_page = Page::<Size4KiB>::containing_address(start);
        let end_page_inclusive = Page::<Size4KiB>::containing_address(end_inclusive);

        for p4_index in u16::from(start_page.p4_index())..=u16::from(end_page_inclusive.p4_index())
        {
            self.mark_p4_index_as_used(PageTableIndex::new(p4_index));
        }
    }

    fn mark_p4_index_as_used(&mut self, p4_index: PageTableIndex) {
        self.entry_state[usize::from(p4_index)] = true;
    }

    pub fn mark_segments<'a>(&mut self, segments: impl Iterator<Item = ProgramHeader<'a>>) {
        for segment in segments.filter(|s| s.mem_size() > 0) {
            self.mark_range_as_used(segment.virtual_addr(), segment.mem_size());
        }
    }

    pub fn get_free_entries(&mut self, num: u64) -> PageTableIndex {
        let mut free_entries = self
            .entry_state
            .windows(num as usize)
            .enumerate()
            .filter(|(_, entries)| entries.iter().all(|used| !used))
            .map(|(idx, _)| idx);

        let idx = free_entries.next().unwrap();

        for i in 0..(num as usize) {
            self.entry_state[idx + i] = true;
        }

        PageTableIndex::new(idx.try_into().unwrap())
    }

    pub fn get_free_address(&mut self, size: u64) -> VirtAddr {
        const LEVEL_4_SIZE: u64 = 4096 * 512 * 512 * 512;

        let level_4_entries = size.div_ceil(LEVEL_4_SIZE);
        let base = Page::from_page_table_indices_1gib(
            self.get_free_entries(level_4_entries),
            PageTableIndex::new(0),
        )
        .start_address();

        base
    }
}
