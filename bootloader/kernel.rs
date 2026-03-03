use bootloader::memory::EarlyFrameAllocator;
use bootloader::paging::UsedLevel4Entries;
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::align_up;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::Translate;
use x86_64::structures::paging::mapper::MappedFrame;
use x86_64::structures::paging::mapper::TranslateResult;
use xmas_elf::ElfFile;
use xmas_elf::header;
use xmas_elf::program;
use xmas_elf::program::ProgramHeader;
use xmas_elf::program::Type;

const COPIED: PageTableFlags = PageTableFlags::BIT_9;

struct Loader<'a> {
    kernel: &'a ElfFile<'a>,
    inner: Inner<'a>,
}

struct Inner<'a> {
    kernel_offset: PhysAddr,
    page_table: &'a mut OffsetPageTable<'static>,
    frame_allocator: &'a mut EarlyFrameAllocator,
}

impl<'a> Loader<'a> {
    fn new(
        kernel: &'a ElfFile<'a>,
        page_table: &'a mut OffsetPageTable<'static>,
        frame_allocator: &'a mut EarlyFrameAllocator,
        used_entries: &'a mut UsedLevel4Entries,
    ) -> Self {
        let kernel_offset = PhysAddr::new(&kernel.input[0] as *const u8 as u64);
        used_entries.mark_segments(kernel.program_iter());

        Loader {
            kernel,
            inner: Inner {
                kernel_offset,
                page_table,
                frame_allocator,
            },
        }
    }

    fn load_segments(&mut self) {
        if self.kernel.header.pt2.type_().as_type() != header::Type::Executable {
            panic!("ELF file type must be EXEC");
        }

        for ph in self.kernel.program_iter() {
            if ph.get_type().unwrap() == program::Type::Dynamic {
                panic!("Dynamic segments are not supported");
            }

            if ph.get_type().unwrap() != Type::Load {
                continue;
            }

            self.inner.handle_load_segment(ph);
        }
    }

    fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.kernel.header.pt2.entry_point())
    }
}

impl<'a> Inner<'a> {
    fn handle_load_segment(&mut self, segment: ProgramHeader) {
        log::info!("Handling segment: {:#x?}", segment);

        let phys_start_addr = self.kernel_offset + segment.offset();
        let start_frame: PhysFrame = PhysFrame::containing_address(phys_start_addr);
        let end_frame: PhysFrame =
            PhysFrame::containing_address(phys_start_addr + segment.file_size() - 1u64);

        let virt_start_addr = VirtAddr::new(segment.virtual_addr());
        let start_page: Page = Page::containing_address(virt_start_addr);

        for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
            let offset = frame - start_frame;
            let page = start_page + offset;
            let flusher = unsafe {
                self.page_table
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        self.frame_allocator,
                    )
                    .unwrap()
            };
            flusher.ignore();
        }

        if segment.mem_size() > segment.file_size() {
            self.handle_bss_section(&segment);
        }
    }

    fn handle_bss_section(&mut self, segment: &ProgramHeader) {
        let virt_start_addr = VirtAddr::new(segment.virtual_addr());
        let mem_size = segment.mem_size();
        let file_size = segment.file_size();

        let zero_start = virt_start_addr + file_size;
        let zero_end = virt_start_addr + mem_size;

        type PageArray = [u64; Size4KiB::SIZE as usize / 8];
        const ZERO_ARRAY: PageArray = [0; Size4KiB::SIZE as usize / 8];

        let data_bytes_before_zero = zero_start.as_u64() & 0xfff;
        if data_bytes_before_zero != 0 {
            let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
            let new_frame = unsafe { self.make_mut(last_page) };
            let new_bytes_ptr = new_frame.start_address().as_u64() as *mut u8;
            unsafe {
                core::ptr::write_bytes(
                    new_bytes_ptr.add(data_bytes_before_zero as usize),
                    0,
                    (Size4KiB::SIZE - data_bytes_before_zero) as usize,
                );
            }
        }

        let start_page: Page =
            Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
        let end_page = Page::containing_address(zero_end - 1u64);
        for page in Page::range_inclusive(start_page, end_page) {
            let frame = self.frame_allocator.allocate_frame().unwrap();
            let frame_ptr = frame.start_address().as_u64() as *mut PageArray;
            unsafe { frame_ptr.write(ZERO_ARRAY) };

            let flusher = unsafe {
                self.page_table
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        self.frame_allocator,
                    )
                    .unwrap()
            };
            flusher.ignore();
        }
    }

    unsafe fn make_mut(&mut self, page: Page) -> PhysFrame {
        let (frame, flags) = match self.page_table.translate(page.start_address()) {
            TranslateResult::Mapped {
                frame,
                offset: _,
                flags,
            } => (frame, flags),
            TranslateResult::NotMapped => panic!("{:?} is not mapped", page),
            TranslateResult::InvalidFrameAddress(_) => unreachable!(),
        };

        let frame = if let MappedFrame::Size4KiB(frame) = frame {
            frame
        } else {
            unreachable!()
        };

        if flags.contains(COPIED) {
            return frame;
        }

        let new_frame = self.frame_allocator.allocate_frame().unwrap();
        let frame_ptr = frame.start_address().as_u64() as *const u8;
        let new_frame_ptr = new_frame.start_address().as_u64() as *mut u8;
        unsafe {
            core::ptr::copy_nonoverlapping(frame_ptr, new_frame_ptr, Size4KiB::SIZE as usize);
        }

        self.page_table.unmap(page).unwrap().1.ignore();
        let new_flags = flags | COPIED;
        unsafe {
            self.page_table
                .map_to(page, new_frame, new_flags, self.frame_allocator)
                .unwrap()
                .ignore();
        }

        new_frame
    }
}

pub fn load_kernel<'a>(
    kernel: &ElfFile<'a>,
    page_table: &'a mut OffsetPageTable<'static>,
    frame_allocator: &'a mut EarlyFrameAllocator,
    used_entries: &'a mut UsedLevel4Entries,
) -> VirtAddr {
    let mut loader = Loader::new(kernel, page_table, frame_allocator, used_entries);
    loader.load_segments();

    loader.entry_point()
}

pub fn calc_memory_requirements(elf: &ElfFile) -> (u64, u64) {
    let max_addr = elf
        .program_iter()
        .filter(|h| matches!(h.get_type(), Ok(Type::Load)))
        .map(|h| h.virtual_addr() + h.mem_size())
        .max()
        .unwrap_or(0);
    let min_addr = elf
        .program_iter()
        .filter(|h| matches!(h.get_type(), Ok(Type::Load)))
        .map(|h| h.virtual_addr())
        .min()
        .unwrap_or(0);

    let size = max_addr - min_addr;

    (size, min_addr)
}
