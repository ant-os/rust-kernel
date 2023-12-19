pub type BitmapChar = [u16; 8];
pub type BitmapFont = [BitmapChar; 128];

#[macro_export]
macro_rules! include_font {
    ($file:expr) => {
        include!($file) as BitmapFont
    };
}

pub trait DisplayChar {
    fn is_set(&self, x: usize, y: usize) -> bool;
}

impl DisplayChar for BitmapChar {
    fn is_set(&self, x: usize, y: usize) -> bool {
        (self[y] & 1 << (x as i8)) != 0
    }
}
