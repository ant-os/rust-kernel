use core::ops::Deref;

use alloc::boxed::Box;

//pub mod buffer;
// pub mod buffer_manager;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(u32);

const CHANNEL_MASK: u32 = 0b11000000;

#[repr(packed)]
#[derive(Debug, PartialEq, Eq)]
pub struct RGBA{
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8
}

impl Color{
    #[inline(always)]
    pub const fn new(code: u32) -> Self{
        Self(code)
    }

    #[inline(always)]
    pub const fn rgb(&self) -> (u8, u8, u8){
        ((self.0 & (CHANNEL_MASK >> 0)) as u8, (self.0 & (CHANNEL_MASK >> 2)) as u8, (self.0 & (CHANNEL_MASK >> 4)) as u8)
    }

    #[inline(always)]
    pub const fn rgba(&self) -> (u8, u8, u8, u8){
        ((self.0 & (CHANNEL_MASK >> 0)) as u8, (self.0 & (CHANNEL_MASK >> 2)) as u8, (self.0 & (CHANNEL_MASK >> 4)) as u8, (self.0 & (CHANNEL_MASK >> 6)) as u8)
    }

    #[inline(always)]
    pub const fn from_rgb(r: u8, b: u8, g: u8) -> Self{
        Self::new(((r as u32) << 6) | ((r as u32) << 4) | ((r as u32) << 2) | 0x000000FF)
    }

    #[inline(always)]
    pub const fn inner(&self) -> u32{
        self.0
    }


}

impl Deref for Color{
    type Target = RGBA;

    #[inline]
    fn deref(&'_ self) -> &'_ Self::Target {
        let rgba = self.rgba();
        unsafe { core::mem::transmute(self) }
    }
}