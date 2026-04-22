#[allow(unsafe_op_in_unsafe_fn)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[allow(private_bounds)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub use root::*;
}

pub use bindings::{std as cpp_std, Dynarmic as cpp};
use std::ops::{Deref, DerefMut};

#[repr(transparent)]
#[allow(unused)]
pub struct TypeInfoPtr(*const ());
unsafe impl Send for TypeInfoPtr {}
unsafe impl Sync for TypeInfoPtr {}

#[cfg(target_env = "msvc")]
pub const VTABLE_DIFF: usize = 0;

#[cfg(not(target_env = "msvc"))]
pub const VTABLE_DIFF: usize = 16;

#[repr(C)]
pub struct InternalCallbacks<T> {
    pub vtable: *const (),
    pub ptr: *mut T,                  // pointer to Callbacks impl
}

/// Wrapper struct for a mutable reference to [Callbacks](crate::a64::Callbacks).
#[repr(transparent)]
pub struct CallbackRef<T>(InternalCallbacks<T>);

impl<T> Deref for CallbackRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.ptr }
    }
}

impl<T> DerefMut for CallbackRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.ptr }
    }
}

const _: () = assert!(std::mem::offset_of!(InternalCallbacks<u8>, vtable) == 0);
const _: () = assert!(std::mem::offset_of!(InternalCallbacks<u8>, ptr) == 8);
const _: () = assert!(size_of::<CallbackRef<u8>>() == size_of::<InternalCallbacks<u8>>());

pub extern "C" fn usercallbacks_destructor() {
    panic!(
        "Dynarmic attempted to call UserCallbacks destructor; UserCallbacks should ALWAYS be owned by Rust code"
    )
}

// A32

pub type A32VAddr = cpp::A32::VAddr;

#[repr(C)]
pub struct A32CallbacksVTable<T> {
    #[cfg(not(target_env = "msvc"))]
    pub offset_to_top: isize, // our inheritence is fake, we're essentially making UserCallbacks but with real functions, so this should always be 0 as UserCallbacks and TranslateCallbacks have no fields
    #[cfg(not(target_env = "msvc"))]
    pub typeinfo: TypeInfoPtr,

    // TranslateCallbacks

    // https://github.com/rust-lang/rust/issues/38258
    #[cfg(not(target_env = "msvc"))]
    pub memory_read_code: unsafe extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> crate::CppOptional<u32>,
    #[cfg(target_env = "msvc")]
    pub memory_read_code: unsafe extern "C" fn(&mut CallbackRef<T>, *mut crate::CppOptional<u32>, A32VAddr),

    pub pre_code_read_hook: extern "C" fn(&mut CallbackRef<T>, bool, A32VAddr, &cpp::A32::IREmitter) -> bool,
    pub pre_code_translation_hook: extern "C" fn(&mut CallbackRef<T>, bool, A32VAddr, &cpp::A32::IREmitter),
    pub get_ticks_for_code: extern "C" fn(&mut CallbackRef<T>, bool, A32VAddr, u32) -> u64,

    // these functions should never be called; UserCallbacks should always be owned by Rust
    pub cpp_destructor: extern "C" fn(),
    #[cfg(not(target_env = "msvc"))]
    pub itanium_destructor: extern "C" fn(),

    // UserCallbacks
    pub memory_read_8: extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> u8,
    pub memory_read_16: extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> u16,
    pub memory_read_32: extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> u32,
    pub memory_read_64: extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> u64,
    pub memory_write_8: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u8),
    pub memory_write_16: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u16),
    pub memory_write_32: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u32),
    pub memory_write_64: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u64),
    pub memory_write_exclusive_8: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u8, u8) -> bool,
    pub memory_write_exclusive_16: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u16, u16) -> bool,
    pub memory_write_exclusive_32: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u32, u32) -> bool,
    pub memory_write_exclusive_64: extern "C" fn(&mut CallbackRef<T>, A32VAddr, u64, u64) -> bool,

    pub is_readonly_memory: extern "C" fn(&mut CallbackRef<T>, A32VAddr) -> bool,
    pub interpreter_fallback: extern "C" fn(&mut CallbackRef<T>, A32VAddr, usize),
    pub call_svc: extern "C" fn(&mut CallbackRef<T>, u32),
    pub exception_raised: extern "C" fn(&mut CallbackRef<T>, A32VAddr, cpp::A32::Exception),
    pub instruction_synchronization_barrier_raised: extern "C" fn(&mut CallbackRef<T>),
    pub add_ticks: extern "C" fn(&mut CallbackRef<T>, u64),
    pub get_ticks_remaining: extern "C" fn(&mut CallbackRef<T>) -> u64,
}

#[repr(C)]
pub struct A32Config<T> {
    pub callbacks: *mut InternalCallbacks<T>,
    pub processor_id: usize,
    pub global_monitor: *mut cpp::ExclusiveMonitor,
    #[doc = " Select the architecture version to use.\n There are minor behavioural differences between versions."]
    pub arch_version: cpp::A32::ArchVersion,
    #[doc = " This selects other optimizations than can't otherwise be disabled by setting other\n configuration options. This includes:\n - IR optimizations\n - Block linking optimizations\n - RSB optimizations\n This is intended to be used for debugging."]
    pub optimizations: crate::OptimizationFlag,
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
    pub fastmem_pointer: crate::CppOptional<usize>,
    #[doc = " Determines if instructions that pagefault should cause recompilation of that block\n with fastmem disabled.\n Recompiled code will use the page_table if this is available, otherwise memory\n accesses will hit the memory callbacks."]
    pub recompile_on_fastmem_failure: bool,
    #[doc = " Determines if we should use the above fastmem_pointer for exclusive reads and\n writes. On x64, dynarmic currently relies on x64 cmpxchg semantics which may not\n provide fully accurate emulation."]
    pub fastmem_exclusive_access: bool,
    #[doc = " Determines if exclusive access instructions that pagefault should cause\n recompilation of that block with fastmem disabled. Recompiled code will use memory\n callbacks."]
    pub recompile_on_exclusive_fastmem_failure: bool,
    pub coprocessors: [crate::CppSharedPtr<cpp::A32::Coprocessor>; 16],
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

// A64

pub type A64VAddr = cpp::A64::VAddr;
use crate::a64::VAddr;
use crate::internal::cpp::A64::{DataCacheOperation, InstructionCacheOperation};

#[repr(C)]
pub struct A64CallbacksVTable<T> {
    #[cfg(not(target_env = "msvc"))]
    pub offset_to_top: isize, // our inheritence is fake, we're essentially making UserCallbacks but with real functions, so this should always be 0 as UserCallbacks has no fields
    #[cfg(not(target_env = "msvc"))]
    pub typeinfo: TypeInfoPtr,

    // these functions should never be called; UserCallbacks should always be owned by Rust
    pub cpp_destructor: extern "C" fn(),
    #[cfg(not(target_env = "msvc"))]
    pub itanium_destructor: extern "C" fn(),

    // https://github.com/rust-lang/rust/issues/38258
    #[cfg(not(target_env = "msvc"))]
    pub memory_read_code: unsafe extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> crate::CppOptional<u32>,
    #[cfg(target_env = "msvc")]
    pub memory_read_code: unsafe extern "C" fn(&mut CallbackRef<T>, *mut crate::CppOptional<u32>, A64VAddr),

    pub memory_read_8: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> u8,
    pub memory_read_16: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> u16,
    pub memory_read_32: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> u32,
    pub memory_read_64: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> u64,
    pub memory_read_128: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> u128,
    pub memory_write_8: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u8),
    pub memory_write_16: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u16),
    pub memory_write_32: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u32),
    pub memory_write_64: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u64),
    pub memory_write_128: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u128),
    pub memory_write_exclusive_8: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u8, u8) -> bool,
    pub memory_write_exclusive_16: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u16, u16) -> bool,
    pub memory_write_exclusive_32: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u32, u32) -> bool,
    pub memory_write_exclusive_64: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u64, u64) -> bool,
    pub memory_write_exclusive_128: extern "C" fn(&mut CallbackRef<T>, A64VAddr, u128, u128) -> bool,

    pub is_readonly_memory: extern "C" fn(&mut CallbackRef<T>, A64VAddr) -> bool,
    pub interpreter_fallback: extern "C" fn(&mut CallbackRef<T>, A64VAddr, usize),
    pub call_svc: extern "C" fn(&mut CallbackRef<T>, u32),
    pub exception_raised: extern "C" fn(&mut CallbackRef<T>, A64VAddr, cpp::A64::Exception),
    pub data_cache_operation_raised: extern "C" fn(&mut CallbackRef<T>, DataCacheOperation, VAddr),
    pub instruction_cache_operation_raised: extern "C" fn(&mut CallbackRef<T>, InstructionCacheOperation, VAddr),
    pub instruction_synchronization_barrier_raised: extern "C" fn(&mut CallbackRef<T>),
    pub add_ticks: extern "C" fn(&mut CallbackRef<T>, u64),
    pub get_ticks_remaining: extern "C" fn(&mut CallbackRef<T>) -> u64,
    pub get_cntpct: extern "C" fn(&mut CallbackRef<T>) -> u64,
}

#[repr(C)]
pub struct A64Config<T> {
    pub callbacks: *mut InternalCallbacks<T>,
    pub processor_id: usize,
    pub global_monitor: *mut super::ExclusiveMonitor,
    #[doc = " This selects other optimizations than can't otherwise be disabled by setting other\n configuration options. This includes:\n - IR optimizations\n - Block linking optimizations\n - RSB optimizations\n This is intended to be used for debugging."]
    pub optimizations: super::OptimizationFlag,
    #[doc = " This enables unsafe optimizations that reduce emulation accuracy in favour of speed.\n For safety, in order to enable unsafe optimizations you have to set BOTH this flag\n AND the appropriate flag bits above.\n The prefered and tested mode for this library is with unsafe optimizations disabled."]
    pub unsafe_optimizations: bool,
    #[doc = " When set to true, UserCallbacks::DxfxataCacheOperationRaised will be called when any\n data cache instruction is executed. Notably DC ZVA will not implicitly do anything.\n When set to false, UserCallbacks::DataCacheOperationRaised will never be called.\n Executing DC ZVA in this mode will result in zeros being written to memory."]
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
    pub tpidrro_el0: *mut u64,
    #[doc = " Pointer to where TPIDR_EL0 is stored. This pointer will be inserted into\n emitted code."]
    pub tpidr_el0: *mut u64,
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
    pub fastmem_pointer: crate::CppOptional<usize>,
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
