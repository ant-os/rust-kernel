pub mod io;
pub mod macros;

#[macro_export]
macro_rules! __asm{
    {
        $($code:block),*
    } => {
        asm!(stringify!($code),*)
    }
}