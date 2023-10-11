use crate::device::{Device, GeneralDevice};

struct Terminal;

impl GeneralDevice for Terminal{
    fn as_device(&self) -> Device<'_>{
        Device::General(self)
    }
}