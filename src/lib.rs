#[cfg(test)]
pub mod tests;
pub(crate) mod internal;
pub mod a32;
pub mod a64;

use std::marker::PhantomData;
use std::mem::MaybeUninit;

pub(crate) use internal::cpp_std::{allocator as cpp_allocator, string as cpp_string};

/// Rust struct representing C++ `std::vector<T, Allocator>`.
/// This type cannot be constructed in Rust.
/// # Safety
/// - This type **cannot** be used to hold any type other than [u128], [u64], and C++ `std::string`.
/// - This type may be used with [std::mem::transmute] to convert to/from a useable `std::vector<T>` type of another crate.
// todo: implement conversions for cxx::CxxVector?
#[repr(C)]
pub struct CppVector<T: CppVectorElement, Allocator: CppVectorAllocator> {
    __alloc: PhantomData<Allocator>,
    __data: [*mut T; 3],
}

unsafe trait CppVectorElement: Sized {
    unsafe fn __dynarmic_drop(vec: &mut CppVector<Self, internal::cpp_std::allocator>);
}
unsafe trait CppVectorAllocator {}

unsafe impl CppVectorElement for internal::cpp_std::string {
    unsafe fn __dynarmic_drop(vec: &mut CppVector<Self, internal::cpp_std::allocator>) {
        unsafe {
            internal::cpp::destroy_vec_cppstring(std::mem::transmute(vec));
        }
    }
}
unsafe impl CppVectorElement for u64 {
    unsafe fn __dynarmic_drop(vec: &mut CppVector<Self, internal::cpp_std::allocator>) {
        unsafe {
            internal::cpp::destroy_vec_u64(vec);
        }
    }
}
unsafe impl CppVectorElement for u128 {
    unsafe fn __dynarmic_drop(vec: &mut CppVector<Self, internal::cpp_std::allocator>) {
        unsafe {
            internal::cpp::destroy_vec_u128(vec);
        }
    }
}
unsafe impl CppVectorAllocator for internal::cpp_std::allocator {}

impl<T: CppVectorElement, Allocator: CppVectorAllocator> Drop for CppVector<T, Allocator> {
    fn drop(&mut self) {
        unsafe {
            T::__dynarmic_drop(std::mem::transmute(self));
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
pub struct CppOptional<T: Sized> {
    __data: MaybeUninit<T>,
    __bool: bool,
}

impl From<usize> for CppOptional<usize> {
    fn from(value: usize) -> Self {
        // bindgen can't pick between usize and uintptr, so to make this work across implementations we transmute anyway
        // safety: std::mem::transmute has to confirm at compile time that usize and the parameter value (usize/u64, depending on impl) are the same size
        unsafe {
            let mut optional: MaybeUninit<CppOptional<usize>> = MaybeUninit::uninit();
            internal::cpp::new_optional_usize(
                std::mem::transmute(&mut optional),
                std::mem::transmute(value),
            );
            optional.assume_init()
        }
    }
}

impl From<u64> for CppOptional<usize> {
    fn from(value: u64) -> Self {
        // bindgen can't pick between usize and uintptr, so to make this work across implementations we transmute anyway
        // safety: std::mem::transmute has to confirm at compile time that u64 and the parameter value (usize/u64, depending on impl) are the same size
        unsafe {
            let mut optional: MaybeUninit<CppOptional<usize>> = MaybeUninit::uninit();
            internal::cpp::new_optional_usize(
                std::mem::transmute(&mut optional),
                std::mem::transmute(value),
            );
            optional.assume_init()
        }
    }
}

impl From<u32> for CppOptional<u32> {
    fn from(value: u32) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CppOptional<u32>> = MaybeUninit::uninit();
            internal::cpp::new_optional_u32(optional.as_mut_ptr(), value);
            optional.assume_init()
        }
    }
}

/// Rust struct representing C++ `std::shared_ptr<T>`.
/// This type can only be constructed in Rust with [Coprocessor].
/// # Safety
/// - This type **will not** run [Drop] (or its C++ destructor) for its T value. This implementation assumes that dynarmic will drop shared_ptr<T>, and dropping in Rust will only release this type, potentially causing memory leaks.
/// - This type may be used with [std::mem::transmute] to convert to/from another `std::shared_ptr<T>` FFI type of another crate.
#[repr(C)]
pub struct CppSharedPtr<T: Sized> {
    __data: *mut T,
    __control: *mut (),
}

impl CppSharedPtr<a32::Coprocessor> {
    pub fn new(coprocessor: a32::Coprocessor) -> Self {
        unsafe {
            let mut optional: MaybeUninit<CppSharedPtr<a32::Coprocessor>> = MaybeUninit::uninit();
            internal::cpp::new_coprocessor(
                optional.as_mut_ptr(),
                std::mem::transmute(&coprocessor),
            ); // safety: new_coprocessor won't modify coprocessor
            optional.assume_init()
        }
    }
}

impl Default for CppSharedPtr<a32::Coprocessor> {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        size_of::<usize>() == size_of::<u64>(),
        "Dynarmic only supports aarch64 and x86_64 architectures."
    );
    assert!(
        internal::cpp::CompilerConstants_Optional_U32 == size_of::<CppOptional<u32>>(),
        "Failed to verify size of type std::optional<u32>"
    );
    assert!(
        internal::cpp::CompilerConstants_Optional_U64 == size_of::<CppOptional<u64>>(),
        "Failed to verify size of type std::optional<u64>"
    );
    assert!(
        internal::cpp::CompilerConstants_Optional_USize == size_of::<CppOptional<usize>>(),
        "Failed to verify size of type std::optional<usize>"
    );
    assert!(
        internal::cpp::CompilerConstants_A32_UserConfig == size_of::<internal::A32Config<u32>>(),
        "Failed to verify size of type a32::UserConfig"
    );
    assert!(
        internal::cpp::CompilerConstants_A64_UserConfig == size_of::<internal::A64Config<u32>>(),
        "Failed to verify size of type a64::UserConfig"
    );
    assert!(
        internal::cpp::CompilerConstants_SharedPtr == size_of::<CppSharedPtr<a32::Coprocessor>>(),
        "Failed to verify size of type cpp_shared_ptr"
    )
};

use bitflags::bitflags;
pub use internal::cpp::{ExclusiveMonitor, HaltReason};

bitflags! {
    #[repr(transparent)]
    pub struct OptimizationFlag: u32 {
        #[doc = "This optimization avoids dispatcher lookups by allowing emitted basic blocks to jump\n directly to other basic blocks if the destination PC is predictable at JIT-time.\n This is a safe optimization."]
        const BlockLinking = 1;
        #[doc = "This optimization avoids dispatcher lookups by emulating a return stack buffer. This\n allows for function returns and syscall returns to be predicted at runtime.\n This is a safe optimization."]
        const ReturnStackBuffer = 2;
        #[doc = "This optimization enables a two-tiered dispatch system.\n A fast dispatcher (written in assembly) first does a look-up in a small MRU cache.\n If this fails, it falls back to the usual slower dispatcher.\n This is a safe optimization."]
        const FastDispatch = 4;
        #[doc = "This is an IR optimization. This optimization eliminates unnecessary emulated CPU state\n context lookups.\n This is a safe optimization."]
        const GetSetElimination = 8;
        #[doc = "This is an IR optimization. This optimization does constant propagation.\n This is a safe optimization."]
        const ConstProp = 16;
        #[doc = "This is enables miscellaneous safe IR optimizations."]
        const MiscIROpt = 32;
        #[doc = "This is an UNSAFE optimization that reduces accuracy of fused multiply-add operations.\n This unfuses fused instructions to improve performance on host CPUs without FMA support."]
        const Unsafe_UnfuseFMA = 65536;
        #[doc = "This is an UNSAFE optimization that reduces accuracy of certain floating-point instructions.\n This allows results of FRECPE and FRSQRTE to have **less** error than spec allows."]
        const Unsafe_ReducedErrorFP = 131072;
        #[doc = "This is an UNSAFE optimization that causes floating-point instructions to not produce correct NaNs.\n This may also result in inaccurate results when instructions are given certain special values."]
        const Unsafe_InaccurateNaN = 262144;
        #[doc = "This is an UNSAFE optimization that causes ASIMD floating-point instructions to be run with incorrect\n rounding modes. This may result in inaccurate results with all floating-point ASIMD instructions."]
        const Unsafe_IgnoreStandardFPCRValue = 524288;
        #[doc = "This is an UNSAFE optimization that causes the global monitor to be ignored. This may\n result in unexpected behaviour in multithreaded scenarios, including but not limited\n to data races and deadlocks."]
        const Unsafe_IgnoreGlobalMonitor = 1048576;

        const AllSafe = 0x0000FFFF;
        const None = 0;
    }
}