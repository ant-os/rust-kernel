pub mod io;
pub mod macros;
pub use limine::*;

pub type Unit = ();

#[macro_export]
macro_rules! __asm{
    {
        $($code:block),*
    } => {
        asm!(stringify!($code),*)
    }
}

#[macro_export]
macro_rules! decl_uninit {
    ($($name:ident => $typ:ty),*) => {
        $(
            static mut $name: core::mem::MaybeUninit::<$typ> = core::mem::MaybeUninit::uninit();
        )*
    };
}

#[macro_export]
macro_rules! assign_uninit{
    {$name:ident($typ:ty) <= $val:expr} =>{
        *(unsafe {&mut $name}) = core::mem::MaybeUninit::<$typ>::new(($val));
    };
}

#[macro_export]
macro_rules! debug{
    ($($msg:expr),*) => {
        unsafe{
        $(
            crate::DEBUG_LINE.unsafe_write_string($msg);
        );
        *
    }
    }
}

pub macro endl() {
    "\n\r"
}
