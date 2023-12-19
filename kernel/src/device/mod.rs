pub mod block;
pub mod character;
pub mod input;
pub mod network;

pub enum Device<'r> {
    Character(&'r dyn character::UnsafeCharacterDevice),
    Block(&'r dyn block::UnsafeBlockDevice),
    Network(&'r dyn network::UnsafeNetworkDevice),
    Input(&'r dyn input::UnsafeInputDevice),
    General(&'r dyn GeneralDevice),
}

pub trait GeneralDevice {
    fn as_device(&self) -> crate::device::Device<'_>;
}
