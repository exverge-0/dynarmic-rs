use std::mem::MaybeUninit;

#[repr(C)]
pub struct CxxVector<T: Sized> {
    __data: [*mut T; 3],
}

#[repr(C)]
pub struct CxxOptional<T: Sized> {
    __data: MaybeUninit<T>,
    __bool: bool,
}

impl From<usize> for CxxOptional<usize> {
    fn from(value: usize) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CxxOptional<usize>> = MaybeUninit::uninit();
            new_optional_usize(optional.as_mut_ptr(), value);
            optional.assume_init()
        }
    }
}

impl From<u64> for CxxOptional<usize> {
    fn from(value: u64) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CxxOptional<usize>> = MaybeUninit::uninit();
            new_optional_usize(optional.as_mut_ptr(), value as _);
            optional.assume_init()
        }
    }
}

impl From<u32> for CxxOptional<u32> {
    fn from(value: u32) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CxxOptional<u32>> = MaybeUninit::uninit();
            new_optional_u32(optional.as_mut_ptr(), value);
            optional.assume_init()
        }
    }
}

#[repr(C)]
pub struct CxxSharedPtr<T: Sized> {
    __data: *mut MaybeUninit<T>,
    __control: *mut (),
}

impl CxxSharedPtr<crate::a32::Coprocessor> {
    pub fn new(coprocessor: crate::a32::Coprocessor) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CxxSharedPtr<crate::a32::Coprocessor>> = MaybeUninit::uninit();
            new_coprocessor(
                optional.as_mut_ptr(),
                &coprocessor as *const _,
            );
            optional.assume_init()
        }
    }
}

impl Default for CxxSharedPtr<crate::a32::Coprocessor> {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub use bindings::*;
mod bindings {
    use crate::cxx::{CxxOptional, CxxSharedPtr};
    use crate::CallbackRef;

    unsafe extern "C-unwind" {
        pub fn new_optional_usize(
            out: *mut CxxOptional<usize>,
            s: usize,
        );
        pub fn new_optional_u32(
            out: *mut CxxOptional<u32>,
            s: u32,
        );
        pub fn new_coprocessor(
            out: *mut CxxSharedPtr<crate::a32::Coprocessor>,
            ptr: *const crate::a32::Coprocessor,
        );
        
        pub fn new_a32_jit(conf: *mut crate::a32::DynarmicConfig<u8>) -> *mut crate::a32::Jit;
        
        pub fn new_a64_jit(conf: *mut crate::a64::DynarmicConfig<u8>) -> *mut crate::a64::Jit;
        pub fn delete_a64_jit(ptr: *mut crate::a64::Jit);
    }
    #[inline(always)]
    pub unsafe fn new_a32_jit_t<T>(conf: &mut crate::a32::DynarmicConfig<T>, cb: &mut CallbackRef<T>) -> *mut crate::a32::Jit {
        unsafe {
            conf.callbacks = cb as *mut _;

            new_a32_jit((conf as *mut crate::a32::DynarmicConfig<T>).cast())
        }
    }
    #[inline(always)]
    pub unsafe fn new_a64_jit_t<T>(conf: &mut crate::a64::DynarmicConfig<T>, cb: &mut CallbackRef<T>) -> *mut crate::a64::Jit {
        unsafe {
            conf.callbacks = cb as *mut _;

            new_a64_jit((conf as *mut crate::a64::DynarmicConfig<T>).cast())
        }
    }
}

const _: () = {
    assert!(
        size_of::<CxxOptional<u32>>() == 8,
        "Failed to verify size of type std::optional<u32>"
    );
    assert!(
        size_of::<CxxOptional<u64>>() == 16,
        "Failed to verify size of type std::optional<u64>"
    );
    assert!(
        size_of::<CxxOptional<usize>>() == 16,
        "Failed to verify size of type std::optional<usize>"
    );
    assert!(
        size_of::<crate::a32::DynarmicConfig<u32>>() == 368,
        "Failed to verify size of type a32::UserConfig"
    );
    assert!(
        size_of::<crate::a64::DynarmicConfig<u32>>() == 144,
        "Failed to verify size of type a64::UserConfig"
    );
    assert!(
        size_of::<CxxSharedPtr<crate::a32::Coprocessor>>() == 16,
        "Failed to verify size of type cpp_shared_ptr"
    );
    assert!(
        size_of::<crate::ExclusiveMonitor>() == 56,
        "Failed to verify size of type ExclusiveMonitor"
    );
};

#[repr(transparent)]
#[allow(unused)]
pub struct TypeInfoPtr(*const ());
unsafe impl Send for TypeInfoPtr {}
unsafe impl Sync for TypeInfoPtr {}

#[cfg(target_env = "msvc")]
pub const VTABLE_DIFF: usize = 0;

#[cfg(not(target_env = "msvc"))]
pub const VTABLE_DIFF: usize = 16;

pub extern "C" fn unimplemented_destructor() {
    panic!(
        "Dynarmic attempted to call UserCallbacks destructor; UserCallbacks should ALWAYS be owned by Rust code"
    )
}