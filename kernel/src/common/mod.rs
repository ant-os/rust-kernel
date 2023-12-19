pub mod consts;
pub mod io;
pub mod macros;
use core::{simd::ptr::SimdConstPtr, ptr::NonNull, mem::{size_of_val, size_of}, sync::atomic::AtomicPtr, ops::Deref};

pub use limine::*;
pub mod idt;
pub mod gdt;
pub mod handler;

pub type Unit = ();

#[doc = "A [AtomicPtr] wrapper that implements [Deref]."]
#[repr(transparent)]
struct AtomicRef<T: Sized>{
    inner: AtomicPtr<T>
}

unsafe impl<T: Sized> Sync for AtomicRef<T> {} /// It's just an [AtomicPtr] internally so it's "thread-safe".

impl<T: Sized> AtomicRef<T> {
    #[doc = "Create a new [AtomicRef]."]
    fn new(inner: *mut T) -> Self { Self { inner: AtomicPtr::new(inner) } }
}

impl<T: Sized> Deref for AtomicRef<T> {
    type Target = T;

    #[doc = "Loads(Relaxed) and then Dereferences the pointer stored by the inner [AtomicRef]. **Panics** when the inner pointer is null."]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.load(core::sync::atomic::Ordering::Relaxed)
            .as_ref()
            .expect("AtomicPtr was null.") }
    }
}

#[doc = "Utility Trait"]
pub(crate) unsafe trait TransmuteIntoPointer{
    /// Calls [core::intrinsics::transmute_unchecked<Self, *mut T>].
    #[inline(always)]
    unsafe fn ptr<T: Sized>(self) -> *mut T where Self: Sized,
    {
        core::intrinsics::transmute_unchecked::<Self, *mut T>(self)
    }
}

#[doc = "Utility Trait"]
pub(crate) unsafe trait TransmuteInto<T: Sized>{
    /// Calls [core::intrinsics::transmute_unchecked<Self, T>]
    #[inline(always)]
    unsafe fn transmute(self) -> T where Self: Sized{
        core::intrinsics::transmute_unchecked::<Self, T>(self)
    }
}

unsafe impl TransmuteIntoPointer for usize { /* empty */ }
unsafe impl<T: Sized> TransmuteInto<NonNullPtr<T>> for NonNull<T>{ /* empty */ }

unsafe impl<T: Sized> TransmuteInto<AtomicPtr<T>> for AtomicRef<T> { /* empty */ }

#[cfg(target_pointer_width = "64")]
unsafe impl TransmuteInto<u64> for usize{ /* empty */ }
#[cfg(target_pointer_width = "32")]
unsafe impl TransmuteInto<u32> for usize{ /* empty */ }

unsafe impl<T: ?Sized> TransmuteInto<usize> for &'_ mut T { /* empty */ }
unsafe impl<T: ?Sized> TransmuteInto<usize> for *mut T { /* empty */ }
unsafe impl<T: ?Sized> TransmuteInto<usize> for *const T { /* empty */ }

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
        core::arch::asm!("lidt [{0}]", in(reg) ptr, options(nostack, preserves_flags));
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
            $crate::DEBUG_LINE.unsafe_write_string($msg);
        )
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
            crate::pf_allocator().request_page()? as *mut (),
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


pub(crate) macro __kdebug_newline() {
    debug!("\n")
}

#[macro_export]
macro_rules! kdebug {
    () => {
        crate::__kdebug_newline!()
    };
    ($($arg:tt)*) => {
        $crate::DEBUG_LINE.unsafe_write_string($crate::alloc_impl::format!($($arg)*).as_str())
    };
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
