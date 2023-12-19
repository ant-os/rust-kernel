use alloc::format;

use crate::{device::{Device, GeneralDevice, character::UnsafeCharacterDevice}, common::endl, renderer::{self, Color, Renderer}};

#[derive(Debug, Default)]
pub struct Console{
    pub cursor_pos: (usize, usize),
    pub line_padding: usize
}

impl GeneralDevice for Console{
    fn as_device(&self) -> Device<'_>{
        Device::Character(self)
    }
}

impl UnsafeCharacterDevice for Console{
    unsafe fn read_raw(&self) -> u8 {
        unimplemented!()
    }

    unsafe fn write_raw(&self, data: u8) {
        let _ = super::renderer::Renderer::global().unsafe_draw_char( self.cursor_pos.0 * 16, self.cursor_pos.1 * (16 + self.line_padding), data);
    }


    unsafe fn received(&self) -> bool {
        false
    }

    unsafe fn is_transmit_empty(&self) -> bool {
        true
    }

    unsafe fn test(&self) -> bool {
        true
    }

    unsafe fn init(&mut self) -> bool {
        todo!()
    }

    fn set_mode(&mut self, mode: crate::device::character::CharacterDeviceMode) {
        
    }

    fn get_mode(&self) -> crate::device::character::CharacterDeviceMode {
        crate::device::character::CharacterDeviceMode::Normal
    }
}

pub(crate) static mut KERNEL_CONSOLE: Console = Console::new();

pub const COLORS: [Color; 8] = [0x00000000, 0xFFFFFFFF, 0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFF00, 0xFFFF00FF, 0xFF00FFFF];
impl Console
where
    Self: UnsafeCharacterDevice
{
    pub fn write_str(&mut self, _str: &'_ str) {
      

        for (idx, chr) in _str.chars().enumerate() {
            if self.cursor_pos.0 > super::Renderer::global().dimensions().0 / 8{
                self.newline()
            }
            if self.cursor_pos.1 > super::Renderer::global().dimensions().1{
                self.scroll();
            }
           
            if chr == '\n'{
                self.newline();
                continue;
            }
            unsafe { self.write_raw(chr as u8) }
            self.cursor_pos.0 += 1;
        
        }

    }

    pub fn get_line_padding(&mut self) -> usize{
        16 + self.line_padding
    }

    pub fn print(&mut self, _str: &'_ str) {
        self.write_str(_str);
        self.newline();
    }

    pub fn newline(&mut self){
        self.cursor_pos.1 += 1;
        self.cursor_pos.0 = 0;
    }

    pub fn scroll(&mut self){
        unsafe { Renderer::global_mut().scroll(8, 2)};

        self.cursor_pos.1 -= 1;
    }

    pub const fn new() -> Self{
        Self{
            cursor_pos: (0,0),
            line_padding: 0
        }
    }
}