#![no_std]

extern crate self as bootloader;

use core::arch::asm;
use x86_64::PhysAddr;

#[derive(Debug, Copy, Clone)]
pub struct Framebuffer {
    pub addr: PhysAddr,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bpp: usize,
    pub pixel_format: PixelFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

pub fn hcf() -> ! {
    loop {
        unsafe { asm!("cli; hlt") }
    }
}
