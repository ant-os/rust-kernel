pub enum CharacterDeviceMode {
    Normal,
    Loopback,
}



pub trait UnsafeCharacterDevice
where
    Self: super::GeneralDevice,
{
    unsafe fn read_raw(&self) -> u8;
    unsafe fn write_raw(&self, data: u8);

    unsafe fn received(&self) -> bool;
    unsafe fn is_transmit_empty(&self) -> bool;

    unsafe fn test(&self) -> bool;
    unsafe fn init(&mut self) -> bool;

    fn set_mode(&mut self, mode: CharacterDeviceMode);
    fn get_mode(&self) -> CharacterDeviceMode;
}

pub trait TimedCharacterDevice
where
    Self: UnsafeCharacterDevice,
{
    unsafe fn read(&self) -> u8;
    unsafe fn write(&self, data: u8);
    unsafe fn wait(&self);
}
