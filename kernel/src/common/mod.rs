pub mod consts;
pub mod io;
pub mod macros;
pub use limine::*;
pub mod idt;
pub mod gdt;


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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(2))]
pub struct DescriptorTablePointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u64,
}

#[inline]
pub unsafe fn lidt(ptr: &DescriptorTablePointer) {
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) ptr, options(readonly, nostack, preserves_flags));
    }
}

#[inline]
pub unsafe fn lgdt(ptr: &DescriptorTablePointer) {
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) ptr, options(readonly, nostack, preserves_flags));
    }
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

#[macro_export]
macro_rules! kprint{
    ($($msg:expr),*) => {
        unsafe{
        $(
            crate::KERNEL_CONSOLE.write_str($msg);
        );
        *

        crate::KERNEL_CONSOLE.newline();
    }
    }
}

#[macro_export]
macro_rules! debug_err{
    ($($msg:expr),*) => {
        debug!("ERROR: ", $($msg), *)
    }
}

pub unsafe fn _alloc_frame_as_mut_t<T: Sized>() -> Result<*mut T, super::paging::frame_allocator::Error> {
   /* if (core::intrinsics::size_of::<T>() > (crate::consts::PAGE_SIZE + 1) as usize) {
        unimplemented!();
    }*/

    Ok(unsafe {
        core::intrinsics::transmute_unchecked::<*mut (), *mut T>(
            crate::pf_allocator!().request_page()? as *mut (),
        )
    })
}

#[macro_export]
macro_rules! pf_alloc_syn{
    { alloc $name:ident = $typ:ty {$($fname:ident: $fval:expr),*} } => {
        let mut $name = &mut *(_alloc_frame_as_mut_t::<$typ>());

        $(
            $name.$fname = ($fval);
        )
        *
    }
}

#[allow(missing_fragment_specifier)]
#[macro_export]
macro_rules! make_wrapper {
    { ( $name:ident($($arg_name:ident: $arg_type:ty), *) ==> $target_name:ident) for $typ:ty[<$success:ty, $error:ty>] @ uninit_err = $uninit_variant:expr } => {
        pub fn $name(&mut self, $($arg_name:$arg_type),*) -> Result<$success, $error> {
            if !self.is_initialized() {
                return Err($uninit_variant);
            } else {
                return self.$target_name($($arg_name),*);
            }
        }
    }
}

#[allow(missing_fragment_specifier)]
#[macro_export]
macro_rules! split_and_invoke_macro{
    {$_macro:path; $($code:block),*}=> {
        $(
            $_macro! { $code }
        )
        *
    }
}

#[macro_export]
macro_rules! return_if_let {
    (($boolean_wrapper:expr) => $variant:path) => {
        if let Some(_boolean) = $boolean_wrapper {
            _boolean
        } else {
            return Err($variant);
        }
    };
}

#[macro_export]
macro_rules! extra_features{
    {for (_, $condition:expr, $update:expr)$code:block} => {
        while $condition{
            $code
            $update;
        }
    }
}

pub macro endl() {
    "\n"
}
