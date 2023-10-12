pub mod io;
pub mod macros;
pub use limine::*;

#[macro_export]
macro_rules! __asm{
    {
        $($code:block),*
    } => {
        asm!(stringify!($code),*)
    }
}