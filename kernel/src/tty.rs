use crate::{device::{Device, GeneralDevice, character::UnsafeCharacterDevice}, common::endl};

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
        super::renderer::Renderer::global().unsafe_draw_char( 10 + self.cursor_pos.0 * 16, 10 + self.cursor_pos.1 * (16 + self.line_padding), data)
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

impl Console
where
    Self: UnsafeCharacterDevice
{
    pub fn write_str(&mut self, _str: &'_ str) {
        for chr in _str.chars() {
            if chr == '\n'{
                self.newline();
                continue;
            }
            unsafe { self.write_raw(chr as u8) }
            self.cursor_pos.0 += 1;
        }

    }

    pub fn print(&mut self, _str: &'_ str) {
        self.write_str(_str);
        self.write_str(endl!());
    }

    pub fn newline(&mut self){
        self.cursor_pos.1 += 1;
        self.cursor_pos.0 = 0;
    }

    pub const fn new() -> Self{
        Self{
            cursor_pos: (0,0),
            line_padding: 0
        }
    }
}