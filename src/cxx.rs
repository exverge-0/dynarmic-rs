use std::mem::MaybeUninit;

/// Empty struct representing C++'s `std::string` in generic types.
/// This cannot be not be used as an owned value and should only be interacted with using C shims or FFI crates like `cxx`.
#[repr(C)]
pub struct CxxString(*const ());

/// Rust struct representing C++ `std::vector<T, Allocator>`.
/// This type cannot be constructed in Rust.
/// # Safety
/// - This type is used solely with Dynarmic's functions, and cannot be used to hold any type other than [u128], [u64], and [C++ `std::string`](CppString).
/// - This type may be used with [std::mem::transmute] to convert to/from a useable `std::vector<T>` type of another crate.
/// - Though the public interface is generally constant, C++ vectors are implementation specific and unsafe to interact with in Rust.
#[allow(private_bounds)]
#[repr(C)]
pub struct CxxVector<T: CppVectorElement> {
    __data: [*mut T; 3],
}

trait CppVectorElement: Sized {
    /// Reserved for use by [dynarmic] crate. Do not use.
    unsafe fn __cpp_vector_drop(vec: &mut CxxVector<Self>);
}

impl CppVectorElement for CxxString {
    unsafe fn __cpp_vector_drop(vec: &mut CxxVector<Self>) {
        unsafe {
            destroy_vec_cppstring(vec);
        }
    }
}
impl CppVectorElement for u64 {
    unsafe fn __cpp_vector_drop(vec: &mut CxxVector<Self>) {
        unsafe {
            destroy_vec_u64(vec);
        }
    }
}
impl CppVectorElement for u128 {
    unsafe fn __cpp_vector_drop(vec: &mut CxxVector<Self>) {
        unsafe {
            destroy_vec_u128(vec);
        }
    }
}

impl<T: CppVectorElement> Drop for CxxVector<T> {
    fn drop(&mut self) {
        unsafe {
            T::__cpp_vector_drop(self);
        }
    }
}

/// Rust struct representing C++ `std::optional<T>`.
/// This type can be constructed from [usize], [u64], or [u32] using [From].
/// # Safety
/// - The size of this type optional<T> is only confirmed to match C++ with T is [usize], [u64], or [u32]. Certain implementations may differ in size/alignment for other types.
/// - This type **will not** run [Drop] (or its C++ destructor) for its T types and should not be used for other types.
/// - This type may be used with [std::mem::transmute] to convert to/from another `std::optional<T>` FFI type of another crate.
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

/// Rust struct representing C++ `std::shared_ptr<T>`.
/// This type only supports construction with [crate::a32::Coprocessor] in Rust, though in theory it should work with any type.
/// # Safety
/// - This type **will not** run [Drop] (or its C++ destructor) for its T value. This implementation assumes that dynarmic will drop shared_ptr<T>, and dropping in Rust will not release the underlying pointer, potentially causing memory leaks.
/// - This type may be used with [std::mem::transmute] to convert to/from another `std::shared_ptr<T>` FFI type of another crate.
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
    use crate::cxx::{CxxOptional, CxxSharedPtr, CxxString, CxxVector};
    use crate::CallbackRef;

    unsafe extern "C" {
        pub fn destroy_vec_cppstring(vec: *mut CxxVector<CxxString>);
        pub fn destroy_vec_u64(
            vec: *mut CxxVector<u64>,
        );
        pub fn destroy_vec_u128(
            vec: *mut CxxVector<u128>,
        );
        
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
        
        pub fn new_a32_jit(conf: *mut crate::internal::A32Config<u8>) -> *mut crate::a32::Jit;
        pub fn delete_a32_jit(ptr: *mut crate::a32::Jit);
        
        pub fn new_a64_jit(conf: *mut crate::internal::A64Config<u8>, ) -> *mut crate::a64::Jit;
        pub fn delete_a64_jit(ptr: *mut crate::a64::Jit);
    }
    #[inline(always)]
    pub unsafe fn new_a32_jit_t<T>(mut conf: crate::internal::A32Config<T>, cb: &mut CallbackRef<T>) -> *mut crate::a32::Jit {
        unsafe {
            conf.callbacks = cb as *mut _;

            new_a32_jit((&mut conf as *mut crate::internal::A32Config<T>).cast())
        }
    }
    #[inline(always)]
    pub unsafe fn new_a64_jit_t<T>(mut conf: crate::internal::A64Config<T>, cb: &mut CallbackRef<T>) -> *mut crate::a64::Jit {
        unsafe {
            conf.callbacks = cb as *mut _;

            new_a64_jit((&mut conf as *mut crate::internal::A64Config<T>).cast())
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
        size_of::<crate::internal::A32Config<u32>>() == 368,
        "Failed to verify size of type a32::UserConfig"
    );
    assert!(
        size_of::<crate::internal::A64Config<u32>>() == 144,
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