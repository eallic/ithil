use bootloader::PAGE_SIZE;
use uefi::boot::MemoryType;
use uefi::mem::memory_map::MemoryMap;
use uefi::mem::memory_map::MemoryMapOwned;
use x86_64::PhysAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;

pub struct EarlyFrameAllocator {
    memory_map: MemoryMapOwned,
    next: usize,
}

impl EarlyFrameAllocator {
    pub fn new(memory_map: MemoryMapOwned) -> Self {
        Self {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.entries();
        let usable_regions = regions.filter(|r| r.ty == MemoryType::CONVENTIONAL);

        let addr_ranges = usable_regions.map(|r| r.phys_start..(r.page_count * PAGE_SIZE as u64));
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(PAGE_SIZE));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    pub fn max_phys_addr(&self) -> PhysAddr {
        self.memory_map
            .entries()
            .map(|r| PhysAddr::new_truncate(r.phys_start + r.page_count * PAGE_SIZE as u64))
            .max()
            .unwrap()
    }
}

unsafe impl FrameAllocator<Size4KiB> for EarlyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
