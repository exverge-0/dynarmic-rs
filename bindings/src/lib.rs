#[cfg(test)]
mod tests;

pub mod internal {
    #![allow(unsafe_op_in_unsafe_fn)]
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]
    #![allow(private_bounds)]

    use std::marker::PhantomData;
    use std::mem::MaybeUninit;

    mod bindings {
        include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    }
    use crate::a32::Coprocessor;
    pub(crate) use bindings::*;
    pub(crate) use root::std::{allocator as cpp_allocator, string as cpp_string};

    /// Rust struct representing C++ `std::vector<T, Allocator>`.
    /// This type cannot be constructed in Rust.
    /// # Safety
    /// - This type **cannot** be used to hold any type other than [u128], [u64], and C++ `std::string`.
    /// - This type may be used with [std::mem::transmute] to convert to/from a useable `std::vector<T>` type of another crate.
    // todo: implement conversions for cxx::CxxVector?
    #[repr(C)]
    pub struct cpp_vector<T: cpp_vector_element, Allocator: cpp_vector_allocator> {
        __alloc: PhantomData<Allocator>,
        __data: [*mut T; 3],
    }

    unsafe trait cpp_vector_element: Sized {
        unsafe fn __dynarmic_drop(vec: &mut cpp_vector<Self, root::std::allocator>);
    }
    unsafe trait cpp_vector_allocator {}

    unsafe impl cpp_vector_element for root::std::string {
        unsafe fn __dynarmic_drop(vec: &mut cpp_vector<Self, root::std::allocator>) {
            unsafe {
                root::Dynarmic::destroy_vec_cppstring(std::mem::transmute(vec));
            }
        }
    }
    unsafe impl cpp_vector_element for u64 {
        unsafe fn __dynarmic_drop(vec: &mut cpp_vector<Self, root::std::allocator>) {
            unsafe {
                root::Dynarmic::destroy_vec_u64(vec);
            }
        }
    }
    unsafe impl cpp_vector_element for u128 {
        unsafe fn __dynarmic_drop(vec: &mut cpp_vector<Self, root::std::allocator>) {
            unsafe {
                root::Dynarmic::destroy_vec_u128(vec);
            }
        }
    }
    unsafe impl cpp_vector_allocator for root::std::allocator {}

    impl<T: cpp_vector_element, Allocator: cpp_vector_allocator> Drop for cpp_vector<T, Allocator> {
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
    pub struct cpp_optional<T: Sized> {
        __data: MaybeUninit<T>,
        __bool: bool,
    }

    impl From<usize> for cpp_optional<usize> {
        fn from(value: usize) -> Self {
            // bindgen can't pick between usize and uintptr, so to make this work across implementations we transmute
            // safety: std::mem::transmute has to confirm at compile time that usize and the parameter value (usize/u64, depending on impl) are the same size
            unsafe {
                let mut optional: MaybeUninit<cpp_optional<usize>> = MaybeUninit::uninit();
                root::Dynarmic::new_optional_usize(std::mem::transmute(&mut optional), std::mem::transmute(value));
                optional.assume_init()
            }
        }
    }

    impl From<u64> for cpp_optional<usize> {
        fn from(value: u64) -> Self {
            // bindgen can't pick between usize and uintptr, so to make this work across implementations we transmute
            // safety: std::mem::transmute has to confirm at compile time that u64 and the parameter value (usize/u64, depending on impl) are the same size
            unsafe {
                let mut optional: MaybeUninit<cpp_optional<usize>> = MaybeUninit::uninit();
                root::Dynarmic::new_optional_usize(std::mem::transmute(&mut optional), std::mem::transmute(value));
                optional.assume_init()
            }
        }
    }

    impl From<u32> for cpp_optional<u32> {
        fn from(value: u32) -> Self {
            unsafe {
                let mut optional: MaybeUninit<cpp_optional<u32>> = MaybeUninit::uninit();
                root::Dynarmic::new_optional_u32(optional.as_mut_ptr(), value);
                optional.assume_init()
            }
        }
    }

    /// Rust struct representing C++ `std::shared_ptr<T>`.
    /// This type can only be constructed in Rust with [Coprocessor].
    /// # Safety
    /// - This type **will not** run [Drop] (or its C++ destructor) for its T value. This implementation assumes that dynarmic will drop shared_ptr<T>, and dropping in Rust will only release this type, potentially causing memory leaks.
    /// - This type may be used with [std::mem::transmute] to convert to/from another `std::shared_ptr<T>` FFI type of another crate.
    // todo: implement conversions for cxx::SharedPtr?
    #[repr(C)]
    pub struct cpp_shared_ptr<T: Sized> {
        __data: *mut T,
        __control: *mut (),
    }

    impl cpp_shared_ptr<Coprocessor> {
        pub fn new(coprocessor: Coprocessor) -> Self {
            unsafe {
                let mut optional: MaybeUninit<cpp_shared_ptr<Coprocessor>> = MaybeUninit::uninit();
                root::Dynarmic::new_coprocessor(optional.as_mut_ptr(), std::mem::transmute(&coprocessor)); // safety: new_coprocessor won't modify coprocessor
                optional.assume_init()
            }
        }
    }

    impl Default for cpp_shared_ptr<Coprocessor> {
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
            root::Dynarmic::CompilerConstants_Optional_U32 == size_of::<cpp_optional<u32>>(),
            "Failed to verify size of type std::optional<u32>"
        );
        assert!(
            root::Dynarmic::CompilerConstants_Optional_U64 == size_of::<cpp_optional<u64>>(),
            "Failed to verify size of type std::optional<u64>"
        );
        assert!(
            root::Dynarmic::CompilerConstants_Optional_USize == size_of::<cpp_optional<usize>>(),
            "Failed to verify size of type std::optional<usize>"
        );
        assert!(
            root::Dynarmic::CompilerConstants_A32_UserConfig == size_of::<super::a32::UserConfig>(),
            "Failed to verify size of type a32::UserConfig"
        );
        assert!(
            root::Dynarmic::CompilerConstants_A64_UserConfig == size_of::<super::a64::UserConfig>(),
            "Failed to verify size of type a64::UserConfig"
        );
        assert!(
            root::Dynarmic::CompilerConstants_SharedPtr
                == size_of::<cpp_shared_ptr<Coprocessor>>(),
            "Failed to verify size of type a32::cpp_shared_ptr"
        )
    };

    #[cfg(target_env = "msvc")]
    const VTABLE_DIFF: usize = 0;

    #[cfg(not(target_env = "msvc"))]
    pub(crate) const VTABLE_DIFF: usize = 16;
}

use bitflags::bitflags;
pub use internal::root::Dynarmic::{ExclusiveMonitor, HaltReason};

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

extern "C" fn usercallbacks_destructor() {
    panic!(
        "Dynarmic attempted to call UserCallbacks destructor; UserCallbacks should ALWAYS be owned by Rust code"
    )
}

#[repr(transparent)]
struct TypeInfoPtr(*const ());
unsafe impl Send for TypeInfoPtr {}
unsafe impl Sync for TypeInfoPtr {}

unsafe extern "C" fn default_true(_: *mut ()) -> bool {
    true
}
unsafe extern "C" fn default_false(_: *mut ()) -> bool {
    false
}
unsafe extern "C" fn default_void(_: *mut ()) {}

pub mod a32 {
    pub use super::internal::root::Dynarmic::A32::{
        ArchVersion, CoprocReg, Coprocessor, Coprocessor__bindgen_vtable as CoprocessorVTable,
        Exception, IREmitter, VAddr,
    };
    use crate::internal::root::Dynarmic::A32::*;
    use crate::internal::root::Dynarmic::{delete_a32_jit, new_a32_jit, A32::Jit as Jit_I};
    use crate::internal::{cpp_optional, cpp_shared_ptr};
    use crate::{HaltReason, OptimizationFlag};
    use std::marker::PhantomData;
    use std::mem::MaybeUninit;

    unsafe extern "C" fn memory_read_code(
        this: *mut UserCallbacks,
        out: *mut cpp_optional<u32>,
        vaddr: VAddr,
    ) {
        unsafe {
            *out = (*((*this).__vtable.byte_sub(crate::internal::VTABLE_DIFF) as *const UserCallbacksVTable))
                .memory_read_32
                .unwrap()(this, vaddr)
                .into()
        }
    }
    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_itanium(this: *mut UserCallbacks, vaddr: VAddr) -> cpp_optional<u32> {
        unsafe {
            let mut uninit: MaybeUninit<cpp_optional<u32>> = MaybeUninit::uninit();
            memory_read_code(this, uninit.as_mut_ptr(), vaddr);
            uninit.assume_init()
        }
    }
    unsafe extern "C" fn get_ticks_for_code(
        _: *mut UserCallbacks,
        _: bool,
        _: VAddr,
        _: u32,
    ) -> u64 {
        1
    }

    unsafe extern "C" fn exception_raised(_: *mut UserCallbacks, addr: VAddr, exc: Exception) {
        panic!("dynarmic-bindings: Unhandled exception '{:?}' at '0x{:X}'", exc, addr)
    }

    unsafe extern "C" fn interpreter_fallback(cb: *mut UserCallbacks, addr: VAddr, num: usize) {
        let func = &*(*cb).__vtable.byte_sub(crate::internal::VTABLE_DIFF).cast::<UserCallbacksVTable>();
        panic!("dynarmic-bindings: Unhandled instruction '0x{:X}' for '{}' instructions at '0x{:X}'", func.memory_read_32.unwrap()(cb, addr), num, addr)
    }

    unsafe extern "C" fn call_svc(_: *mut UserCallbacks, svc: u32) {
        panic!("dynarmic-bindings: Unhandled supervisor call '{}'", svc)
    }

    #[repr(C)]
    pub struct UserCallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        offset_to_top: isize, // our inheritence is fake, we're essentially making UserCallbacks but with real functions, so this should always be 0 as UserCallbacks and TranslateCallbacks have no fields
        #[cfg(not(target_env = "msvc"))]
        typeinfo: crate::TypeInfoPtr,

        // TranslateCallbacks

        // https://github.com/rust-lang/rust/issues/38258
        #[cfg(not(target_env = "msvc"))]
        memory_read_code:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> cpp_optional<u32>>,
        #[cfg(target_env = "msvc")]
        memory_read_code:
            Option<unsafe extern "C" fn(*mut UserCallbacks, *mut cpp_optional<u32>, VAddr)>,

        pre_code_read_hook: Option<unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter) -> bool>,
        pre_code_translation_hook:
            Option<unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter)>,
        get_ticks_for_code:
            Option<unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, u32) -> u64>,

        // these functions should never be called; UserCallbacks should always be owned by Rust
        cpp_destructor: Option<unsafe extern "C" fn()>,
        #[cfg(not(target_env = "msvc"))]
        itanium_destructor: Option<unsafe extern "C" fn()>,

        // UserCallbacks
        memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
        memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
        memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
        memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
        memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
        memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
        memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
        memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
        memory_write_exclusive_8:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool>,
        memory_write_exclusive_16:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool>,
        memory_write_exclusive_32:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool>,
        memory_write_exclusive_64:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool>,
        is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
        interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
        call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
        exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
        instruction_synchronization_barrier_raised:
            Option<unsafe extern "C" fn(*mut UserCallbacks)>,
        add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
        get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
    }

    #[repr(C)]
    pub struct UserCallbacks {
        __vtable: *const *const (),
        __copy: PhantomData<*mut ()>, // prevent UserCallbacks from being Send/Sync
    }

    impl UserCallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        pub const fn new(
            memory_read_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> cpp_optional<u32>,
            >,
            pre_code_read_hook: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter) -> bool,
            >,
            pre_code_translation_hook: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter),
            >,
            get_ticks_for_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, u32) -> u64,
            >,
            memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
            memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
            memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
            memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
            memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
            memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
            memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
            memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
            memory_write_exclusive_8: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool,
            >,
            memory_write_exclusive_16: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool,
            >,
            memory_write_exclusive_32: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool,
            >,
            memory_write_exclusive_64: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool,
            >,
            is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
            interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
            call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
            exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
            instruction_synchronization_barrier_raised: Option<
                unsafe extern "C" fn(*mut UserCallbacks),
            >,
            add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
            get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
        ) -> Self {
            let mut value = Self {
                offset_to_top: 0,
                typeinfo: crate::TypeInfoPtr(std::ptr::null()),
                cpp_destructor: Some(crate::usercallbacks_destructor),
                itanium_destructor: Some(crate::usercallbacks_destructor),
                memory_read_code,
                pre_code_read_hook,
                pre_code_translation_hook,
                get_ticks_for_code,
                memory_read_8,
                memory_read_16,
                memory_read_32,
                memory_read_64,
                memory_write_8,
                memory_write_16,
                memory_write_32,
                memory_write_64,
                memory_write_exclusive_8,
                memory_write_exclusive_16,
                memory_write_exclusive_32,
                memory_write_exclusive_64,
                is_readonly_memory,
                interpreter_fallback,
                call_svc,
                exception_raised,
                instruction_synchronization_barrier_raised,
                add_ticks,
                get_ticks_remaining,
            };
            if value.memory_read_code.is_none() {
                value.memory_read_code = Some(memory_read_code_itanium)
            }
            if value.pre_code_read_hook.is_none() {
                unsafe {
                    value.pre_code_read_hook = Some(std::mem::transmute(
                        crate::default_true as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.pre_code_translation_hook.is_none() {
                unsafe {
                    value.pre_code_translation_hook = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.get_ticks_for_code.is_none() {
                value.get_ticks_for_code = Some(super::a32::get_ticks_for_code);
            }
            if value.memory_write_exclusive_8.is_none() {
                unsafe {
                    value.memory_write_exclusive_8 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_16.is_none() {
                unsafe {
                    value.memory_write_exclusive_16 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_32.is_none() {
                unsafe {
                    value.memory_write_exclusive_32 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_64.is_none() {
                unsafe {
                    value.memory_write_exclusive_64 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.is_readonly_memory.is_none() {
                unsafe {
                    value.is_readonly_memory = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.instruction_synchronization_barrier_raised.is_none() {
                unsafe {
                    value.instruction_synchronization_barrier_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.exception_raised.is_none() {
                value.exception_raised = Some(super::a32::exception_raised);
            }
            if value.interpreter_fallback.is_none() {
                value.interpreter_fallback = Some(super::a32::interpreter_fallback);
            }
            if value.call_svc.is_none() {
                value.call_svc = Some(super::a32::call_svc);
            }
            value
        }

        #[cfg(target_env = "msvc")]
        pub const fn new(
            memory_read_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, *mut cpp_optional<u32>, VAddr),
            >,
            pre_code_read_hook: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter) -> bool,
            >,
            pre_code_translation_hook: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, *mut IREmitter),
            >,
            get_ticks_for_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, bool, VAddr, u32) -> u64,
            >,
            memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
            memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
            memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
            memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
            memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
            memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
            memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
            memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
            memory_write_exclusive_8: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool,
            >,
            memory_write_exclusive_16: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool,
            >,
            memory_write_exclusive_32: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool,
            >,
            memory_write_exclusive_64: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool,
            >,
            is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
            interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
            call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
            exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
            instruction_synchronization_barrier_raised: Option<
                unsafe extern "C" fn(*mut UserCallbacks),
            >,
            add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
            get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
        ) -> Self {
            let mut value = Self {
                cpp_destructor: Some(crate::usercallbacks_destructor),
                memory_read_code,
                pre_code_read_hook,
                pre_code_translation_hook,
                get_ticks_for_code,
                memory_read_8,
                memory_read_16,
                memory_read_32,
                memory_read_64,
                memory_write_8,
                memory_write_16,
                memory_write_32,
                memory_write_64,
                memory_write_exclusive_8,
                memory_write_exclusive_16,
                memory_write_exclusive_32,
                memory_write_exclusive_64,
                is_readonly_memory,
                interpreter_fallback,
                call_svc,
                exception_raised,
                instruction_synchronization_barrier_raised,
                add_ticks,
                get_ticks_remaining,
            };
            if value.memory_read_code.is_none() {
                value.memory_read_code = Some(super::a32::memory_read_code)
            }
            if value.pre_code_read_hook.is_none() {
                unsafe {
                    value.pre_code_read_hook = Some(std::mem::transmute(
                        crate::default_true as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.pre_code_translation_hook.is_none() {
                unsafe {
                    value.pre_code_translation_hook = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.get_ticks_for_code.is_none() {
                value.get_ticks_for_code = Some(super::a32::get_ticks_for_code);
            }
            if value.memory_write_exclusive_8.is_none() {
                unsafe {
                    value.memory_write_exclusive_8 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_16.is_none() {
                unsafe {
                    value.memory_write_exclusive_16 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_32.is_none() {
                unsafe {
                    value.memory_write_exclusive_32 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_64.is_none() {
                unsafe {
                    value.memory_write_exclusive_64 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.is_readonly_memory.is_none() {
                unsafe {
                    value.is_readonly_memory = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.instruction_synchronization_barrier_raised.is_none() {
                unsafe {
                    value.instruction_synchronization_barrier_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.exception_raised.is_none() {
                value.exception_raised = Some(super::a32::exception_raised);
            }
            if value.interpreter_fallback.is_none() {
                value.interpreter_fallback = Some(super::a32::interpreter_fallback);
            }
            if value.call_svc.is_none() {
                value.call_svc = Some(super::a32::call_svc);
            }
            value
        }
    }

    impl UserCallbacks {
        pub fn new(vtable: &'static UserCallbacksVTable) -> Self {
            unsafe {
                Self {
                    // skip typeinfo data on itanium
                    __vtable: std::mem::transmute::<
                        &'static UserCallbacksVTable,
                        *const *const (),
                    >(vtable)
                        .byte_add(crate::internal::VTABLE_DIFF),
                    __copy: PhantomData,
                }
            }
        }
    }

    #[repr(C)]
    pub struct UserConfig<'a> {
        pub callbacks: &'a UserCallbacks,
        pub processor_id: usize,
        pub global_monitor: Option<&'a mut super::ExclusiveMonitor>,
        #[doc = " Select the architecture version to use.\n There are minor behavioural differences between versions."]
        pub arch_version: ArchVersion,
        #[doc = " This selects other optimizations than can't otherwise be disabled by setting other\n configuration options. This includes:\n - IR optimizations\n - Block linking optimizations\n - RSB optimizations\n This is intended to be used for debugging."]
        pub optimizations: OptimizationFlag,
        #[doc = " This enables unsafe optimizations that reduce emulation accuracy in favour of speed.\n For safety, in order to enable unsafe optimizations you have to set BOTH this flag\n AND the appropriate flag bits above.\n The prefered and tested mode for this library is with unsafe optimizations disabled."]
        pub unsafe_optimizations: bool,
        pub page_table: *mut [*mut u8; 1 << (32 - 12)],
        #[doc = " Determines if the pointer in the page_table shall be offseted locally or globally.\n 'false' will access page_table[addr >> bits][addr & mask]\n 'true'  will access page_table[addr >> bits][addr]\n Note: page_table[addr >> bits] will still be checked to verify active pages.\n       So there might be wrongly faulted pages which maps to nullptr.\n       This can be avoided by carefully allocating the memory region."]
        pub absolute_offset_page_table: bool,
        #[doc = " Masks out the first N bits in host pointers from the page table.\n The intention behind this is to allow users of Dynarmic to pack attributes in the\n same integer and update the pointer attribute pair atomically.\n If the configured value is 3, all pointers will be forcefully aligned to 8 bytes."]
        pub page_table_pointer_mask_bits: std::os::raw::c_int,
        #[doc = " Determines if we should detect memory accesses via page_table that straddle are\n misaligned. Accesses that straddle page boundaries will fallback to the relevant\n memory callback.\n This value should be the required access sizes this applies to ORed together.\n To detect any access, use: 8 | 16 | 32 | 64."]
        pub detect_misaligned_access_via_page_table: u8,
        #[doc = " Determines if the above option only triggers when the misalignment straddles a\n page boundary."]
        pub only_detect_misalignment_via_page_table_on_page_boundary: bool,
        pub fastmem_pointer: cpp_optional<usize>,
        #[doc = " Determines if instructions that pagefault should cause recompilation of that block\n with fastmem disabled.\n Recompiled code will use the page_table if this is available, otherwise memory\n accesses will hit the memory callbacks."]
        pub recompile_on_fastmem_failure: bool,
        #[doc = " Determines if we should use the above fastmem_pointer for exclusive reads and\n writes. On x64, dynarmic currently relies on x64 cmpxchg semantics which may not\n provide fully accurate emulation."]
        pub fastmem_exclusive_access: bool,
        #[doc = " Determines if exclusive access instructions that pagefault should cause\n recompilation of that block with fastmem disabled. Recompiled code will use memory\n callbacks."]
        pub recompile_on_exclusive_fastmem_failure: bool,
        pub coprocessors: [cpp_shared_ptr<Coprocessor>; 16],
        #[doc = " When set to true, UserCallbacks::InstructionSynchronizationBarrierRaised will be\n called when an ISB instruction is executed.\n When set to false, ISB will be treated as a NOP instruction."]
        pub hook_isb: bool,
        #[doc = " Hint instructions would cause ExceptionRaised to be called with the appropriate\n argument."]
        pub hook_hint_instructions: bool,
        #[doc = " This option relates to translation. Generally when we run into an unpredictable\n instruction the ExceptionRaised callback is called. If this is true, we define\n definite behaviour for some unpredictable instructions."]
        pub define_unpredictable_behaviour: bool,
        #[doc = " HACK:\n This tells the translator a wall clock will be used, thus allowing it\n to avoid writting certain unnecessary code only needed for cycle timers."]
        pub wall_clock_cntpct: bool,
        #[doc = " This allows accurately emulating protection fault handlers. If true, we check\n for exit after every data memory access by the emulated program."]
        pub check_halt_on_memory_access: bool,
        #[doc = " This option allows you to disable cycle counting. If this is set to false,\n AddTicks and GetTicksRemaining are never called, and no cycle counting is done."]
        pub enable_cycle_counting: bool,
        #[doc = " This option relates to the CPSR.E flag. Enabling this option disables modification\n of CPSR.E by the emulated program, forcing it to 0.\n NOTE: Calling Jit::SetCpsr with CPSR.E=1 while this option is enabled may result\n       in unusual behavior."]
        pub always_little_endian: bool,
        pub code_cache_size: usize,
        #[doc = " Internal use only"]
        pub very_verbose_debugging_output: bool,
    }

    impl<'a> UserConfig<'a> {
        pub fn new(
            callbacks: &'a mut UserCallbacks,
            global_monitor: Option<&'a mut super::ExclusiveMonitor>,
        ) -> UserConfig<'a> {
            Self {
                callbacks,
                processor_id: 0,
                global_monitor,
                arch_version: ArchVersion::v8,
                optimizations: OptimizationFlag::AllSafe,
                unsafe_optimizations: false,
                page_table: std::ptr::null_mut(),
                absolute_offset_page_table: false,
                page_table_pointer_mask_bits: 0,
                detect_misaligned_access_via_page_table: 0,
                only_detect_misalignment_via_page_table_on_page_boundary: false,
                fastmem_pointer: 0usize.into(),
                recompile_on_fastmem_failure: true,
                fastmem_exclusive_access: false,
                recompile_on_exclusive_fastmem_failure: true,
                coprocessors: Default::default(),
                hook_isb: false,
                hook_hint_instructions: false,
                define_unpredictable_behaviour: false,
                wall_clock_cntpct: false,
                check_halt_on_memory_access: false,
                enable_cycle_counting: true,
                always_little_endian: false,
                code_cache_size: 128 * 1024 * 1024,
                very_verbose_debugging_output: false,
            }
        }
    }

    pub struct Jit<'callbacks> {
        ptr: *mut Jit_I,
        lifetime: PhantomData<&'callbacks ()>,
    }

    impl Drop for Jit<'_> {
        fn drop(&mut self) {
            unsafe {
                delete_a32_jit(self.ptr)
            }
        }
    }

    impl<'callbacks> Jit<'callbacks> {
        pub unsafe fn new(mut conf: UserConfig<'callbacks>) -> Jit<'callbacks> {
            unsafe {
                Jit {
                    ptr: new_a32_jit(&mut conf as *mut UserConfig),
                    lifetime: PhantomData,
                }
            }
        }

        /// Runs the emulated CPU.
        /// Cannot be recursively called.
        /// # Safety:
        /// - All instructions and memory addresses inputted must be valid. Invalid addresses/instructions will cause dynarmic exceptions, which panic by default.
        #[inline]
        pub fn run(&mut self) -> HaltReason {
            unsafe { Jit_Run(self.ptr) }
        }

        /// Step the emulated CPU for one instruction.
        /// Cannot be recursively called.
        #[inline]
        pub fn step(&mut self) -> HaltReason {
            unsafe { Jit_Step(self.ptr) }
        }

        /// Clears the code cache of all compiled code.
        /// Can be called at any time. Halts execution if called within a callback.
        #[inline]
        pub fn clear_cache(&mut self) {
            unsafe { Jit_ClearCache(self.ptr) }
        }

        /// Reset CPU state to state at startup. Does not clear code cache.
        /// Cannot be called from a callback.
        #[inline]
        pub fn reset(&mut self) {
            unsafe { Jit_Reset(self.ptr) }
        }

        /// Stops execution during [Jit::run].
        #[inline]
        pub fn halt(&mut self, hr: HaltReason) {
            unsafe { Jit_HaltExecution(self.ptr, hr) }
        }

        /// Clears a halt reason from flags.
        #[inline]
        pub unsafe fn clear_halt(&mut self, hr: HaltReason) {
            unsafe { Jit_ClearHalt(self.ptr, hr) }
        }

        /// View general-purpose registers.
        #[inline]
        pub fn get_regs(&self) -> &[u32; 16] {
            unsafe { std::slice::from_raw_parts::<u32>(Jit_Regs(self.ptr).cast(), 16).try_into().unwrap_unchecked() }
        }

        /// Replace general-purpose registers.
        #[inline]
        pub fn set_regs(&mut self, regs: [u32; 16]) {
            unsafe {
                std::ptr::copy_nonoverlapping(regs.as_ptr(), Jit_Regs(self.ptr).cast(), 16);
            }
        }

        /// Get raw FP/SIMD registers in units of u32.
        #[inline]
        pub fn get_extregs(&self) -> &[u32; 64] {
            unsafe { std::slice::from_raw_parts::<u32>(Jit_ExtRegs(self.ptr).cast(), 64).try_into().unwrap_unchecked() }
        }

        /// Replace FP/SIMD registers.
        #[inline]
        pub fn set_extregs(&self, regs: [u32; 64]) {
            unsafe {
                std::ptr::copy_nonoverlapping(regs.as_ptr(), Jit_ExtRegs(self.ptr).cast(), 64);
            }
        }

        #[inline]
        pub fn get_reg(&self, index: usize) -> u32 {
            self.get_regs()[index]
        }

        #[inline]
        pub fn set_reg(&mut self, index: usize, val: u32) {
            unsafe { std::slice::from_raw_parts_mut::<u32>(Jit_Regs(self.ptr).cast(), 16)[index] = val }
        }

        /// Read Stack Pointer
        #[inline]
        pub fn get_sp(&self) -> u32 {
            self.get_reg(13)
        }

        /// Modify Stack Pointer
        #[inline]
        pub fn set_sp(&mut self, sp: u32) {
            self.set_reg(13, sp)
        }

        /// Read Program Counter
        #[inline]
        pub fn get_pc(&self) -> u32 {
            self.get_reg(15)
        }

        /// Modify Program Counter
        #[inline]
        pub fn set_pc(&mut self, pc: u32) {
            self.set_reg(15, pc)
        }

        /// View CPSR
        #[inline]
        pub fn get_cpsr(&self) -> u32 {
            unsafe { Jit_Cpsr(self.ptr) }
        }

        /// Modify CPSR
        #[inline]
        pub fn set_cpsr(&mut self, val: u32) {
            unsafe { Jit_SetCpsr(self.ptr, val) }
        }

        /// View FPSCR
        #[inline]
        pub fn get_fpscr(&self) -> u32 {
            unsafe { Jit_Fpscr(self.ptr) }
        }

        /// Modify FPSCR
        #[inline]
        pub fn set_fpscr(&mut self, val: u32) {
            unsafe { Jit_SetFpscr(self.ptr, val) }
        }

        /// Clears exclusive states for this core.
        #[inline]
        pub fn clear_exclusive_state(&mut self) {
            unsafe { Jit_ClearExclusiveState(self.ptr) }
        }

        /// Returns true if Jit::Run was called but hasn't returned yet.
        /// i.e; we're in a callback
        #[inline]
        pub fn is_executing(&self) -> bool {
            unsafe { (*self.ptr).is_executing }
        }

        /// Dumps the disassembly of all compiled code to stdout.
        #[inline]
        pub fn dump_disassembly(&self) {
            unsafe { Jit_DumpDisassembly(self.ptr) }
        }

        /// Disassemble the instructions following the current pc and return
        /// the resulting instructions as a vector of their string representations.
        #[inline]
        pub fn disassemble(&self) -> crate::internal::cpp_vector<crate::internal::cpp_string, crate::internal::cpp_allocator> {
            unsafe {
                if cfg!(not(target_env = "msvc")) {
                    std::mem::transmute(Jit_Disassemble(self.ptr)) // safety: compile-time checks verify vector size
                } else {
                    // fix function signature to reflect msvc abi
                    let og = Jit_Disassemble as unsafe extern "C" fn(*const Jit_I) -> _;
                    let func: unsafe extern "C" fn(*const Jit_I, *mut crate::internal::cpp_vector<crate::internal::cpp_string, crate::internal::cpp_allocator>) = std::mem::transmute(og);

                    let mut vector = MaybeUninit::uninit();
                    func(self.ptr, vector.as_mut_ptr());
                    vector.assume_init()
                }
            }
        }
    }
}

pub mod a64 {
    pub use super::internal::root::Dynarmic::A64::{
        DataCacheOperation, Exception, InstructionCacheOperation, VAddr,
    };
    use crate::internal::cpp_optional;
    use crate::internal::root::Dynarmic::A64::*;
    use crate::internal::root::Dynarmic::{delete_a64_jit, new_a64_jit, A64::Jit as Jit_I};
    use crate::HaltReason;
    use std::marker::PhantomData;
    use std::mem::MaybeUninit;

    #[repr(C)]
    pub struct UserCallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        offset_to_top: isize, // our inheritence is fake, we're essentially making UserCallbacks but with real functions, so this should always be 0 as UserCallbacks has no fields
        #[cfg(not(target_env = "msvc"))]
        typeinfo: crate::TypeInfoPtr,

        // these functions should never be called; UserCallbacks should always be owned by Rust
        cpp_destructor: Option<unsafe extern "C" fn()>,
        #[cfg(not(target_env = "msvc"))]
        itanium_destructor: Option<unsafe extern "C" fn()>,

        // https://github.com/rust-lang/rust/issues/38258
        #[cfg(not(target_env = "msvc"))]
        memory_read_code:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> cpp_optional<u32>>,
        #[cfg(target_env = "msvc")]
        memory_read_code:
            Option<unsafe extern "C" fn(*mut UserCallbacks, *mut cpp_optional<u32>, VAddr)>,

        memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
        memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
        memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
        memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
        memory_read_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u128>,
        memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
        memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
        memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
        memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
        memory_write_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128)>,
        memory_write_exclusive_8:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool>,
        memory_write_exclusive_16:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool>,
        memory_write_exclusive_32:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool>,
        memory_write_exclusive_64:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool>,
        memory_write_exclusive_128:
            Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128, u128) -> bool>,
        is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
        interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
        call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
        exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
        data_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
        instruction_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
        instruction_synchronization_barrier_raised:
            Option<unsafe extern "C" fn(*mut UserCallbacks)>,
        add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
        get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
        get_cntpct: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
    }
    #[repr(C)]
    pub struct UserCallbacks {
        __vtable: *const *const (),
        __copy: std::marker::PhantomData<*mut ()>, // prevent UserCallbacks from being Send/Sync
    }

    unsafe extern "C" fn memory_read_code(
        this: *mut UserCallbacks,
        out: *mut cpp_optional<u32>,
        vaddr: VAddr,
    ) {
        unsafe {
            *out = (*((*this).__vtable.byte_sub(crate::internal::VTABLE_DIFF) as *const UserCallbacksVTable))
                .memory_read_32
                .unwrap()(this, vaddr)
                .into()
        }
    }

    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_itanium(this: *mut UserCallbacks, vaddr: VAddr) -> cpp_optional<u32> {
        unsafe {
            let mut uninit: MaybeUninit<cpp_optional<u32>> = MaybeUninit::uninit();
            memory_read_code(this, uninit.as_mut_ptr(), vaddr);
            uninit.assume_init()
        }
    }

    unsafe extern "C" fn exception_raised(_: *mut UserCallbacks, addr: VAddr, exc: Exception) {
        panic!("dynarmic-bindings: Unhandled exception '{:?}' at '0x{:X}'", exc, addr)
    }

    unsafe extern "C" fn interpreter_fallback(cb: *mut UserCallbacks, addr: VAddr, num: usize) {
        let func = &*(*cb).__vtable.byte_sub(crate::internal::VTABLE_DIFF).cast::<UserCallbacksVTable>();
        panic!("dynarmic-bindings: Unhandled instruction '0x{:X}' for '{}' instructions at '0x{:X}'", func.memory_read_32.unwrap()(cb, addr), num, addr)
    }

    unsafe extern "C" fn call_svc(_: *mut UserCallbacks, svc: u32) {
        panic!("dynarmic-bindings: Unhandled supervisor call '{}'", svc)
    }

    impl UserCallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        pub const fn new(
            memory_read_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> cpp_optional<u32>,
            >,
            memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
            memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
            memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
            memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
            memory_read_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u128>,
            memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
            memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
            memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
            memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
            memory_write_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128)>,
            memory_write_exclusive_8: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool,
            >,
            memory_write_exclusive_16: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool,
            >,
            memory_write_exclusive_32: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool,
            >,
            memory_write_exclusive_64: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool,
            >,
            memory_write_exclusive_128: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128, u128) -> bool,
            >,
            is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
            interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
            call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
            exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
            data_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
            instruction_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
            instruction_synchronization_barrier_raised: Option<
                unsafe extern "C" fn(*mut UserCallbacks),
            >,
            add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
            get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
            get_cntpct: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
        ) -> UserCallbacksVTable {
            let mut value = Self {
                offset_to_top: 0,
                typeinfo: crate::TypeInfoPtr(std::ptr::null()),
                cpp_destructor: Some(crate::usercallbacks_destructor),
                itanium_destructor: Some(crate::usercallbacks_destructor),
                memory_read_code,
                memory_read_8,
                memory_read_16,
                memory_read_32,
                memory_read_64,
                memory_read_128,
                memory_write_8,
                memory_write_16,
                memory_write_32,
                memory_write_64,
                memory_write_128,
                memory_write_exclusive_8,
                memory_write_exclusive_16,
                memory_write_exclusive_32,
                memory_write_exclusive_64,
                memory_write_exclusive_128,
                is_readonly_memory,
                interpreter_fallback,
                call_svc,
                exception_raised,
                data_cache_operation_raised,
                instruction_cache_operation_raised,
                instruction_synchronization_barrier_raised,
                add_ticks,
                get_ticks_remaining,
                get_cntpct,
            };
            if value.memory_read_code.is_none() {
                value.memory_read_code = Some(memory_read_code_itanium)
            }
            if value.memory_write_exclusive_8.is_none() {
                unsafe {
                    value.memory_write_exclusive_8 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_16.is_none() {
                unsafe {
                    value.memory_write_exclusive_16 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_32.is_none() {
                unsafe {
                    value.memory_write_exclusive_32 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_64.is_none() {
                unsafe {
                    value.memory_write_exclusive_64 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_128.is_none() {
                unsafe {
                    value.memory_write_exclusive_128 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.is_readonly_memory.is_none() {
                unsafe {
                    value.is_readonly_memory = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.instruction_cache_operation_raised.is_none() {
                unsafe {
                    value.instruction_cache_operation_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.data_cache_operation_raised.is_none() {
                unsafe {
                    value.data_cache_operation_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.instruction_synchronization_barrier_raised.is_none() {
                unsafe {
                    value.instruction_synchronization_barrier_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.exception_raised.is_none() {
                value.exception_raised = Some(super::a64::exception_raised);
            }
            if value.interpreter_fallback.is_none() {
                value.interpreter_fallback = Some(super::a64::interpreter_fallback);
            }
            if value.call_svc.is_none() {
                value.call_svc = Some(super::a64::call_svc);
            }
            value
        }

        #[cfg(target_env = "msvc")]
        pub const fn new(
            memory_read_code: Option<
                unsafe extern "C" fn(*mut UserCallbacks, *mut cpp_optional<u32>, VAddr),
            >,
            memory_read_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u8>,
            memory_read_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u16>,
            memory_read_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u32>,
            memory_read_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u64>,
            memory_read_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> u128>,
            memory_write_8: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8)>,
            memory_write_16: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16)>,
            memory_write_32: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32)>,
            memory_write_64: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64)>,
            memory_write_128: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128)>,
            memory_write_exclusive_8: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u8, u8) -> bool,
            >,
            memory_write_exclusive_16: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u16, u16) -> bool,
            >,
            memory_write_exclusive_32: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u32, u32) -> bool,
            >,
            memory_write_exclusive_64: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u64, u64) -> bool,
            >,
            memory_write_exclusive_128: Option<
                unsafe extern "C" fn(*mut UserCallbacks, VAddr, u128, u128) -> bool,
            >,
            is_readonly_memory: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr) -> bool>,
            interpreter_fallback: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, usize)>,
            call_svc: Option<unsafe extern "C" fn(*mut UserCallbacks, u32)>,
            exception_raised: Option<unsafe extern "C" fn(*mut UserCallbacks, VAddr, Exception)>,
            data_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
            instruction_cache_operation_raised: Option<unsafe extern "C" fn(*mut UserCallbacks)>,
            instruction_synchronization_barrier_raised: Option<
                unsafe extern "C" fn(*mut UserCallbacks),
            >,
            add_ticks: Option<unsafe extern "C" fn(*mut UserCallbacks, u64)>,
            get_ticks_remaining: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
            get_cntpct: Option<unsafe extern "C" fn(*mut UserCallbacks) -> u64>,
        ) -> UserCallbacksVTable {
            let mut value = Self {
                cpp_destructor: Some(crate::usercallbacks_destructor),
                memory_read_code,
                memory_read_8,
                memory_read_16,
                memory_read_32,
                memory_read_64,
                memory_read_128,
                memory_write_8,
                memory_write_16,
                memory_write_32,
                memory_write_64,
                memory_write_128,
                memory_write_exclusive_8,
                memory_write_exclusive_16,
                memory_write_exclusive_32,
                memory_write_exclusive_64,
                memory_write_exclusive_128,
                is_readonly_memory,
                interpreter_fallback,
                call_svc,
                exception_raised,
                data_cache_operation_raised,
                instruction_cache_operation_raised,
                instruction_synchronization_barrier_raised,
                add_ticks,
                get_ticks_remaining,
                get_cntpct,
            };
            if value.memory_read_code.is_none() {
                value.memory_read_code = Some(super::a64::memory_read_code)
            }
            if value.memory_write_exclusive_8.is_none() {
                unsafe {
                    value.memory_write_exclusive_8 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_16.is_none() {
                unsafe {
                    value.memory_write_exclusive_16 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_32.is_none() {
                unsafe {
                    value.memory_write_exclusive_32 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_64.is_none() {
                unsafe {
                    value.memory_write_exclusive_64 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.memory_write_exclusive_128.is_none() {
                unsafe {
                    value.memory_write_exclusive_128 = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.is_readonly_memory.is_none() {
                unsafe {
                    value.is_readonly_memory = Some(std::mem::transmute(
                        crate::default_false as unsafe extern "C" fn(*mut ()) -> bool,
                    ))
                }
            }
            if value.instruction_cache_operation_raised.is_none() {
                unsafe {
                    value.instruction_cache_operation_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.data_cache_operation_raised.is_none() {
                unsafe {
                    value.data_cache_operation_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.instruction_synchronization_barrier_raised.is_none() {
                unsafe {
                    value.instruction_synchronization_barrier_raised = Some(std::mem::transmute(
                        crate::default_void as unsafe extern "C" fn(*mut ()),
                    ))
                }
            }
            if value.exception_raised.is_none() {
                value.exception_raised = Some(super::a64::exception_raised);
            }
            if value.interpreter_fallback.is_none() {
                value.interpreter_fallback = Some(super::a64::interpreter_fallback);
            }
            if value.call_svc.is_none() {
                value.call_svc = Some(super::a64::call_svc);
            }
            value
        }
    }

    impl UserCallbacks {
        pub fn new(vtable: &'static UserCallbacksVTable) -> Self {
            unsafe {
                Self {
                    // skip typeinfo data
                    __vtable: std::mem::transmute::<
                        &'static UserCallbacksVTable,
                        *const *const (),
                    >(vtable)
                        .byte_add(crate::internal::VTABLE_DIFF),
                    __copy: PhantomData,
                }
            }
        }
    }

    #[repr(C)]
    pub struct UserConfig<'a> {
        pub callbacks: &'a mut UserCallbacks,
        pub processor_id: usize,
        pub global_monitor: *mut super::ExclusiveMonitor,
        #[doc = " This selects other optimizations than can't otherwise be disabled by setting other\n configuration options. This includes:\n - IR optimizations\n - Block linking optimizations\n - RSB optimizations\n This is intended to be used for debugging."]
        pub optimizations: super::OptimizationFlag,
        #[doc = " This enables unsafe optimizations that reduce emulation accuracy in favour of speed.\n For safety, in order to enable unsafe optimizations you have to set BOTH this flag\n AND the appropriate flag bits above.\n The prefered and tested mode for this library is with unsafe optimizations disabled."]
        pub unsafe_optimizations: bool,
        #[doc = " When set to true, UserCallbacks::DataCacheOperationRaised will be called when any\n data cache instruction is executed. Notably DC ZVA will not implicitly do anything.\n When set to false, UserCallbacks::DataCacheOperationRaised will never be called.\n Executing DC ZVA in this mode will result in zeros being written to memory."]
        pub hook_data_cache_operations: bool,
        #[doc = " When set to true, UserCallbacks::InstructionSynchronizationBarrierRaised will be\n called when an ISB instruction is executed.\n When set to false, ISB will be treated as a NOP instruction."]
        pub hook_isb: bool,
        #[doc = " When set to true, UserCallbacks::ExceptionRaised will be called when any hint\n instruction is executed."]
        pub hook_hint_instructions: bool,
        #[doc = " Counter-timer frequency register. The value of the register is not interpreted by\n dynarmic."]
        pub cntfrq_el0: u32,
        #[doc = " CTR_EL0<27:24> is log2 of the cache writeback granule in words.\n CTR_EL0<23:20> is log2 of the exclusives reservation granule in words.\n CTR_EL0<19:16> is log2 of the smallest data/unified cacheline in words.\n CTR_EL0<15:14> is the level 1 instruction cache policy.\n CTR_EL0<3:0> is log2 of the smallest instruction cacheline in words."]
        pub ctr_el0: u32,
        #[doc = " DCZID_EL0<3:0> is log2 of the block size in words\n DCZID_EL0<4> is 0 if the DC ZVA instruction is permitted."]
        pub dczid_el0: u32,
        #[doc = " Pointer to where TPIDRRO_EL0 is stored. This pointer will be inserted into\n emitted code."]
        pub tpidrro_el0: Option<&'a u64>,
        #[doc = " Pointer to where TPIDR_EL0 is stored. This pointer will be inserted into\n emitted code."]
        pub tpidr_el0: Option<&'a u64>,
        #[doc = " Pointer to the page table which we can use for direct page table access.\n If an entry in page_table is null, the relevant memory callback will be called.\n If page_table is nullptr, all memory accesses hit the memory callbacks."]
        pub page_table: *mut *mut std::ffi::c_void,
        #[doc = " Declares how many valid address bits are there in virtual addresses.\n Determines the size of page_table. Valid values are between 12 and 64 inclusive.\n This is only used if page_table is not nullptr."]
        pub page_table_address_space_bits: usize,
        #[doc = " Masks out the first N bits in host pointers from the page table.\n The intention behind this is to allow users of Dynarmic to pack attributes in the\n same integer and update the pointer attribute pair atomically.\n If the configured value is 3, all pointers will be forcefully aligned to 8 bytes."]
        pub page_table_pointer_mask_bits: std::os::raw::c_int,
        #[doc = " Determines what happens if the guest accesses an entry that is off the end of the\n page table. If true, Dynarmic will silently mirror page_table's address space. If\n false, accessing memory outside of page_table bounds will result in a call to the\n relevant memory callback.\n This is only used if page_table is not nullptr."]
        pub silently_mirror_page_table: bool,
        #[doc = " Determines if the pointer in the page_table shall be offseted locally or globally.\n 'false' will access page_table[addr >> bits][addr & mask]\n 'true'  will access page_table[addr >> bits][addr]\n Note: page_table[addr >> bits] will still be checked to verify active pages.\n       So there might be wrongly faulted pages which maps to nullptr.\n       This can be avoided by carefully allocating the memory region."]
        pub absolute_offset_page_table: bool,
        #[doc = " Determines if we should detect memory accesses via page_table that straddle are\n misaligned. Accesses that straddle page boundaries will fallback to the relevant\n memory callback.\n This value should be the required access sizes this applies to ORed together.\n To detect any access, use: 8 | 16 | 32 | 64 | 128."]
        pub detect_misaligned_access_via_page_table: u8,
        #[doc = " Determines if the above option only triggers when the misalignment straddles a\n page boundary."]
        pub only_detect_misalignment_via_page_table_on_page_boundary: bool,
        #[doc = " Fastmem Pointer\n This should point to the beginning of a 2^page_table_address_space_bits bytes\n address space which is in arranged just like what you wish for emulated memory to\n be. If the host page faults on an address, the JIT will fallback to calling the\n MemoryRead*MemoryWrite* callbacks."]
        pub fastmem_pointer: cpp_optional<usize>,
        #[doc = " Determines if instructions that pagefault should cause recompilation of that block\n with fastmem disabled.\n Recompiled code will use the page_table if this is available, otherwise memory\n accesses will hit the memory callbacks."]
        pub recompile_on_fastmem_failure: bool,
        #[doc = " Declares how many valid address bits are there in virtual addresses.\n Determines the size of fastmem arena. Valid values are between 12 and 64 inclusive.\n This is only used if fastmem_pointer is set."]
        pub fastmem_address_space_bits: usize,
        #[doc = " Determines what happens if the guest accesses an entry that is off the end of the\n fastmem arena. If true, Dynarmic will silently mirror fastmem's address space. If\n false, accessing memory outside of fastmem bounds will result in a call to the\n relevant memory callback.\n This is only used if fastmem_pointer is set."]
        pub silently_mirror_fastmem: bool,
        #[doc = " Determines if we should use the above fastmem_pointer for exclusive reads and\n writes. On x64, dynarmic currently relies on x64 cmpxchg semantics which may not\n provide fully accurate emulation."]
        pub fastmem_exclusive_access: bool,
        #[doc = " Determines if exclusive access instructions that pagefault should cause\n recompilation of that block with fastmem disabled. Recompiled code will use memory\n callbacks."]
        pub recompile_on_exclusive_fastmem_failure: bool,
        #[doc = " This option relates to translation. Generally when we run into an unpredictable\n instruction the ExceptionRaised callback is called. If this is true, we define\n definite behaviour for some unpredictable instructions."]
        pub define_unpredictable_behaviour: bool,
        #[doc = " HACK:\n This tells the translator a wall clock will be used, thus allowing it\n to avoid writting certain unnecessary code only needed for cycle timers."]
        pub wall_clock_cntpct: bool,
        #[doc = " This allows accurately emulating protection fault handlers. If true, we check\n for exit after every data memory access by the emulated program."]
        pub check_halt_on_memory_access: bool,
        #[doc = " This option allows you to disable cycle counting. If this is set to false,\n AddTicks and GetTicksRemaining are never called, and no cycle counting is done."]
        pub enable_cycle_counting: bool,
        pub code_cache_size: usize,
        #[doc = " Internal use only"]
        pub very_verbose_debugging_output: bool,
    }

    impl<'a> UserConfig<'a> {
        pub fn new(callbacks: &'a mut UserCallbacks) -> UserConfig<'a> {
            Self {
                callbacks,
                processor_id: 0,
                global_monitor: std::ptr::null_mut(),
                optimizations: crate::OptimizationFlag::AllSafe,
                unsafe_optimizations: false,
                hook_data_cache_operations: false,
                hook_isb: false,
                hook_hint_instructions: false,
                cntfrq_el0: 600000000,
                ctr_el0: 0x8444c004,
                dczid_el0: 4,
                tpidrro_el0: None,
                tpidr_el0: None,
                page_table: std::ptr::null_mut(),
                page_table_address_space_bits: 36,
                page_table_pointer_mask_bits: 0,
                silently_mirror_page_table: true,
                absolute_offset_page_table: false,
                detect_misaligned_access_via_page_table: 0,
                only_detect_misalignment_via_page_table_on_page_boundary: false,
                fastmem_pointer: 0usize.into(),
                recompile_on_fastmem_failure: true,
                fastmem_address_space_bits: 36,
                silently_mirror_fastmem: true,
                fastmem_exclusive_access: false,
                recompile_on_exclusive_fastmem_failure: true,
                define_unpredictable_behaviour: false,
                wall_clock_cntpct: false,
                check_halt_on_memory_access: false,
                enable_cycle_counting: true,
                code_cache_size: 128 * 1024 * 1024,
                very_verbose_debugging_output: false,
            }
        }
    }

    pub struct Jit<'callbacks> {
        ptr: *mut Jit_I,
        lifetime: PhantomData<&'callbacks ()>, // lifetime for UserCallbacks
    }

    impl Drop for Jit<'_> {
        fn drop(&mut self) {
            unsafe {
                delete_a64_jit(self.ptr);
            }
        }
    }

    impl<'callbacks> Jit<'callbacks> {
        /// Creates a new A64 Jit instance.
        /// # Safety
        /// - A valid [UserConfig] and [UserCallbacks] must be inputted. This ensures the safety of any other functions for Jit.
        pub unsafe fn new(mut conf: UserConfig<'callbacks>) -> Jit<'callbacks> {
            unsafe {
                Jit {
                    ptr: new_a64_jit(&mut conf as *mut UserConfig),
                    lifetime: PhantomData,
                }
            }
        }

        /// Runs the emulated CPU.
        /// Cannot be recursively called.
        #[inline]
        pub fn run(&mut self) -> HaltReason {
            unsafe { Jit_Run(self.ptr) }
        }

        /// Step the emulated CPU for one instruction.
        /// Cannot be recursively called.
        #[inline]
        pub fn step(&mut self) -> HaltReason {
            unsafe { Jit_Step(self.ptr) }
        }

        /// Clears the code cache of all compiled code.
        /// Can be called at any time. Halts execution if called within a callback.
        #[inline]
        pub fn clear_cache(&mut self) {
            unsafe { Jit_ClearCache(self.ptr) }
        }

        /// Reset CPU state to state at startup. Does not clear code cache.
        /// Cannot be called from a callback.
        #[inline]
        pub fn reset(&mut self) {
            unsafe { Jit_Reset(self.ptr) }
        }

        /// Stops execution during [Jit::run].
        #[inline]
        pub unsafe fn halt(&mut self, hr: HaltReason) {
            unsafe { Jit_HaltExecution(self.ptr, hr) }
        }

        /// Clears a halt reason from flags.
        #[inline]
        pub unsafe fn clear_halt(&mut self, hr: HaltReason) {
            unsafe { Jit_ClearHalt(self.ptr, hr) }
        }

        /// Read Stack Pointer
        #[inline]
        pub fn get_sp(&self) -> u64 {
            unsafe { Jit_GetSP(self.ptr) }
        }

        /// Modify Stack Pointer
        #[inline]
        pub fn set_sp(&mut self, sp: u64) {
            unsafe { Jit_SetSP(self.ptr, sp) }
        }

        /// Read Program Counter
        #[inline]
        pub fn get_pc(&self) -> u64 {
            unsafe { Jit_GetPC(self.ptr) }
        }

        /// Modify Program Counter
        #[inline]
        pub fn set_pc(&mut self, pc: u64) {
            unsafe { Jit_SetPC(self.ptr, pc) }
        }

        /// Read general-purpose register. (GPR)
        #[inline]
        pub fn get_reg(&self, index: usize) -> u64 {
            unsafe { Jit_GetRegister(self.ptr, index as _) }
        }

        /// Read the low 32-bits of a GPR.
        #[inline]
        pub fn get_wreg(&self, index: usize) -> u32 {
            self.get_reg(index) as u32
        }

        /// Modify general-purpose register. (GPR)
        #[inline]
        pub fn set_reg(&mut self, index: usize, val: u64) {
            unsafe { Jit_SetRegister(self.ptr, index as _, val) }
        }

        /// Read all general-purpose registers.
        #[inline]
        pub fn get_regs(&self) -> [u64; 31] {
            unsafe {
                // todo: bindgen can't generate std::array (even though it's the same size as a C/Rust one?) so the return value of GetRegisters is just u8..
                let og = Jit_GetRegisters as unsafe extern "C" fn(*const Jit_I) -> u8;
                let func: unsafe extern "C" fn(*const Jit_I) -> [u64; 31] = std::mem::transmute(og);

                func(self.ptr)
            }
        }

        /// Replace all general-purpose registers.
        #[inline]
        pub fn set_regs(&mut self, regs: &[u64; 31]) {
            unsafe { Jit_SetRegisters(self.ptr, regs.as_ptr().cast()) }
        }

        /// Read floating point and SIMD register.
        #[inline]
        pub fn get_vector(&self, index: usize) -> u128 {
            unsafe { Jit_GetVector(self.ptr, index as _) }
        }

        /// Modify floating point/SIMD register. (GPR)
        #[inline]
        pub fn set_vector(&mut self, index: usize, val: u128) {
            unsafe { Jit_SetVector(self.ptr, index as _, val) }
        }

        /// Read all floating point and SIMD registers.
        #[inline]
        pub fn get_vectors(&self) -> [u128; 32] {
            unsafe {
                // bindgen can't generate std::array (even though it's the same size as a C/Rust one?) so the return value of GetRegisters is just u8..
                let og = Jit_GetVectors as unsafe extern "C" fn(*const Jit_I) -> u8;
                let func: unsafe extern "C" fn(*const Jit_I) -> [u128; 32] = std::mem::transmute(og);

                func(self.ptr)
            }
        }

        /// Replace all general-purpose registers.
        #[inline]
        pub fn set_vectors(&mut self, regs: &[u128; 32]) {
            unsafe { Jit_SetRegisters(self.ptr, regs.as_ptr().cast()) }
        }

        /// View FPCR
        #[inline]
        pub fn get_fpcr(&self) -> u32 {
            unsafe { Jit_GetFpcr(self.ptr) }
        }

        /// Modify FPCR
        #[inline]
        pub fn set_fpcr(&mut self, val: u32) {
            unsafe { Jit_SetFpcr(self.ptr, val) }
        }

        /// View PSTATE
        #[inline]
        pub fn get_pstate(&self) -> u32 {
            unsafe { Jit_GetPstate(self.ptr) }
        }

        /// Modify FPCR
        #[inline]
        pub fn set_pstate(&mut self, val: u32) {
            unsafe { Jit_SetPstate(self.ptr, val) }
        }

        /// Clears exclusive states for this core.
        #[inline]
        pub fn clear_exclusive_state(&mut self) {
            unsafe { Jit_ClearExclusiveState(self.ptr) }
        }

        /// Returns true if Jit::Run was called but hasn't returned yet.
        /// i.e; we're in a callback
        #[inline]
        pub fn is_executing(&self) -> bool {
            unsafe { Jit_IsExecuting(self.ptr) }
        }

        /// Dumps the disassembly of all compiled code to stdout.
        #[inline]
        pub fn dump_disassembly(&self) {
            unsafe { Jit_DumpDisassembly(self.ptr) }
        }

        /// Disassemble the instructions following the current pc and return
        /// the resulting instructions as a vector of their string representations.
        #[inline]
        pub fn disassemble(&self) -> crate::internal::cpp_vector<crate::internal::cpp_string, crate::internal::cpp_allocator> {
            unsafe {
                if cfg!(not(target_env = "msvc")) {
                    std::mem::transmute(Jit_Disassemble(self.ptr)) // safety: compile-time checks verify vector size
                } else {
                    // fix function signature to reflect msvc abi
                    let og = Jit_Disassemble as unsafe extern "C" fn(*const Jit_I) -> _;
                    let func: unsafe extern "C" fn(*const Jit_I, *mut crate::internal::cpp_vector<crate::internal::cpp_string, crate::internal::cpp_allocator>) = std::mem::transmute(og);

                    let mut vector = MaybeUninit::uninit();
                    func(self.ptr, vector.as_mut_ptr());
                    vector.assume_init()
                }
            }
        }
    }
}
