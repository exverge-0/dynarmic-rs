pub(crate) mod cxx;
pub mod a32;
pub mod a64;

pub use cxx::{CxxOptional, CxxVector, CxxSharedPtr};
pub use a32::Dynarmic as DynarmicA32;
pub use a64::Dynarmic as DynarmicA64;

use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

/// Integer type used for memory reads and writes.
pub trait GuestInt : num_traits::PrimInt + num_traits::Unsigned {}
impl<T> GuestInt for T where T: num_traits::PrimInt + num_traits::Unsigned {}

/// Wrapper struct for a mutable reference to a type implementing dynarmic Callbacks.
#[repr(C)]
pub struct CallbackRef<T> {
    pub(crate) vtable: *const (),
    pub(crate) ptr: *mut T,
}

impl<T> CallbackRef<T> {
    const _PTR_ASSERT: () = {
        assert!(std::mem::offset_of!(CallbackRef<T>, vtable) == 0);
    };
}

impl<T> Deref for CallbackRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for CallbackRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

#[repr(transparent)]
pub struct OptimizationFlag(u32);
impl OptimizationFlag {

    /// This optimization avoids dispatcher lookups by allowing emitted basic blocks to jump
    /// directly to other basic blocks if the destination PC is predictable at JIT-time.
    /// This is a safe optimization.
    pub const BLOCK_LINKING: Self = Self(1);
    
    /// This optimization avoids dispatcher lookups by emulating a return stack buffer. This
    /// allows for function returns and syscall returns to be predicted at runtime.
    /// This is a safe optimization.
    pub const RETURN_STACK_BUFFER : Self = Self(2);
    
    /// This optimization enables a two-tiered dispatch system.
    /// A fast dispatcher (written in assembly) first does a look-up in a small MRU cache.
    /// If this fails, it falls back to the usual slower dispatcher.
    /// This is a safe optimization.
    pub const FAST_DISPATCH : Self = Self(4);
    
    /// This is an IR optimization. This optimization eliminates unnecessary emulated CPU state
    /// context lookups.
    /// This is a safe optimization.
    pub const GET_SET_ELIMINATION : Self = Self(8);
    
    /// This is an IR optimization. This optimization does constant propagation.
    /// This is a safe optimization.
    pub const CONST_PROP : Self = Self(16);
    
    /// This is enables miscellaneous safe IR optimizations.
    pub const MISC_IR_OPT : Self = Self(32);
    
    /// This is an UNSAFE optimization that reduces accuracy of fused multiply-add operations.
    /// This unfuses fused instructions to improve performance on host CPUs without FMA support.
    pub const UNSAFE_UNFUSE_FMA : Self = Self(65536);
    
    /// This is an UNSAFE optimization that reduces accuracy of certain floating-point instructions.
    /// This allows results of FRECPE and FRSQRTE to have **less** error than spec allows.
    pub const UNSAFE_REDUCED_ERROR_FP : Self = Self(131072);
    
    /// This is an UNSAFE optimization that causes floating-point instructions to not produce correct NaNs.
    /// This may also result in inaccurate results when instructions are given certain special values.
    pub const UNSAFE_INACCURATENAN : Self = Self(262144);
    
    /// This is an UNSAFE optimization that causes ASIMD floating-point instructions to be run with incorrect
    /// rounding modes. This may result in inaccurate results with all floating-point ASIMD instructions.
    pub const UNSAFE_IGNORE_STANDARD_FPCR_VALUE : Self = Self(524288);
    
    /// This is an UNSAFE optimization that causes the global monitor to be ignored. This may
    /// result in unexpected behaviour in multithreaded scenarios, including but not limited
    /// to data races and deadlocks.
    pub const UNSAFE_IGNORE_GLOBALMONITOR : Self = Self(1048576);
    

    /// All safe optimizations. 
    /// The default used by dynarmic.
    pub const ALL : Self = Self(0x0000FFFF);
    pub const NONE : Self = Self(0);
}

#[repr(u32)]
#[derive(Debug)]
pub enum HaltReason {
    Step = 1,
    CacheInvalidation = 2,
    MemoryAbort = 4,
    UserDefined1 = 16777216,
    UserDefined2 = 33554432,
    UserDefined3 = 67108864,
    UserDefined4 = 134217728,
    UserDefined5 = 268435456,
    UserDefined6 = 536870912,
    UserDefined7 = 1073741824,
    UserDefined8 = 2147483648,
}

#[repr(C)]
pub struct ExclusiveMonitor {
    spinlock: i32,
    exclusive_addr: CxxVector<a64::VAddr>,
    exclusive_val: CxxVector<u128>
}

// todo: can we implement ReadAndMark and DoExclusiveOperation?
impl ExclusiveMonitor {
    pub fn new(processor_count: usize) -> Self {
        unsafe extern "C-unwind" {
            fn ExclusiveMonitor_ExclusiveMonitor(
                this: *mut ExclusiveMonitor,
                processor_count: usize,
            );
        }
        let mut init = MaybeUninit::<Self>::uninit();
        unsafe {
            ExclusiveMonitor_ExclusiveMonitor(init.as_mut_ptr(), processor_count);
            init.assume_init()
        }
    }
    pub fn clear_processor(&mut self, id: usize) {
        unsafe extern "C-unwind" {
            pub fn ExclusiveMonitor_ClearProcessor(
                this: &mut ExclusiveMonitor,
                processor_id: usize,
            );
        }
        unsafe { ExclusiveMonitor_ClearProcessor(self, id) }
    }
    pub fn clear(&mut self) {
        unsafe extern "C-unwind" {
            pub fn ExclusiveMonitor_Clear(this: &mut ExclusiveMonitor);
        }
        unsafe { ExclusiveMonitor_Clear(self) }
    }
    pub fn get_processor_count(&mut self) -> usize {
        unsafe extern "C-unwind" {
            pub fn ExclusiveMonitor_GetProcessorCount(this: &mut ExclusiveMonitor) -> usize;
        }
        unsafe { ExclusiveMonitor_GetProcessorCount(self) }
    }
}