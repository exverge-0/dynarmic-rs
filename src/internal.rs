use crate::*;

#[repr(transparent)]
#[allow(unused)]
pub struct TypeInfoPtr(*const ());
unsafe impl Send for TypeInfoPtr {}
unsafe impl Sync for TypeInfoPtr {}

#[cfg(target_env = "msvc")]
pub const VTABLE_DIFF: usize = 0;

#[cfg(not(target_env = "msvc"))]
pub const VTABLE_DIFF: usize = 16;

pub extern "C" fn usercallbacks_destructor() {
    panic!(
        "Dynarmic attempted to call UserCallbacks destructor; UserCallbacks should ALWAYS be owned by Rust code"
    )
}

// A32

#[repr(C)]
pub struct A32CallbacksVTable<T> {
    #[cfg(not(target_env = "msvc"))]
    pub offset_to_top: isize, // our inheritence is fake, we're essentially making UserCallbacks but with real functions, so this should always be 0 as UserCallbacks and TranslateCallbacks have no fields
    #[cfg(not(target_env = "msvc"))]
    pub typeinfo: TypeInfoPtr,

    // TranslateCallbacks

    // https://github.com/rust-lang/rust/issues/38258
    #[cfg(not(target_env = "msvc"))]
    pub memory_read_code: unsafe extern "C" fn(&CallbackRef<T>, a32::VAddr) -> cxx::CxxOptional<u32>,
    #[cfg(target_env = "msvc")]
    pub memory_read_code: unsafe extern "C" fn(&CallbackRef<T>, *mut cxx::CxxOptional<u32>, a32::VAddr),

    pub pre_code_read_hook: extern "C" fn(&mut CallbackRef<T>, bool, a32::VAddr, &a32::IREmitter) -> bool,
    pub pre_code_translation_hook: extern "C" fn(&mut CallbackRef<T>, bool, a32::VAddr, &a32::IREmitter),
    pub get_ticks_for_code: extern "C" fn(&mut CallbackRef<T>, bool, a32::VAddr, u32) -> u64,

    // these functions should never be called; UserCallbacks should always be owned by Rust
    pub cpp_destructor: extern "C" fn(),
    #[cfg(not(target_env = "msvc"))]
    pub itanium_destructor: extern "C" fn(),

    // UserCallbacks
    pub memory_read_8: extern "C" fn(&CallbackRef<T>, a32::VAddr) -> u8,
    pub memory_read_16: extern "C" fn(&CallbackRef<T>, a32::VAddr) -> u16,
    pub memory_read_32: extern "C" fn(&CallbackRef<T>, a32::VAddr) -> u32,
    pub memory_read_64: extern "C" fn(&CallbackRef<T>, a32::VAddr) -> u64,
    pub memory_write_8: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u8),
    pub memory_write_16: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u16),
    pub memory_write_32: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u32),
    pub memory_write_64: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u64),
    pub memory_write_exclusive_8: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u8, u8) -> bool,
    pub memory_write_exclusive_16: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u16, u16) -> bool,
    pub memory_write_exclusive_32: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u32, u32) -> bool,
    pub memory_write_exclusive_64: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, u64, u64) -> bool,

    pub is_readonly_memory: extern "C" fn(&CallbackRef<T>, a32::VAddr) -> bool,
    pub interpreter_fallback: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, usize),
    pub call_svc: extern "C" fn(&mut CallbackRef<T>, u32),
    pub exception_raised: extern "C" fn(&mut CallbackRef<T>, a32::VAddr, a32::Exception),
    pub instruction_synchronization_barrier_raised: extern "C" fn(&mut CallbackRef<T>),
    pub add_ticks: extern "C" fn(&mut CallbackRef<T>, u64),
    pub get_ticks_remaining: extern "C" fn(&CallbackRef<T>) -> u64,
}

#[repr(C)]
pub struct A32Config<T> {
    pub callbacks: *mut CallbackRef<T>,
    pub processor_id: usize,
    pub global_monitor: *mut ExclusiveMonitor,
    /// Select the architecture version to use.
    /// There are minor behavioural differences between versions.
    pub arch_version: a32::ArchVersion,
    /// This selects other optimizations than can't otherwise be disabled by setting other
    /// configuration options. This includes:
    /// - IR optimizations
    /// - Block linking optimizations
    /// - RSB optimizations
    /// This is intended to be used for debugging.
    pub optimizations: OptimizationFlag,
    /// This enables unsafe optimizations that reduce emulation accuracy in favour of speed.
    /// For safety, in order to enable unsafe optimizations you have to set BOTH this flag
    /// AND the appropriate flag bits above.
    /// The prefered and tested mode for this library is with unsafe optimizations disabled.
    pub unsafe_optimizations: bool,
    pub page_table: *mut [*mut u8; 1 << (32 - 12)],
    /// Determines if the pointer in the page_table shall be offseted locally or globally.
    /// 'false' will access page_table[addr >> bits][addr & mask]
    /// 'true'  will access page_table[addr >> bits][addr]
    /// Note: page_table[addr >> bits] will still be checked to verify active pages.
    ///      So there might be wrongly faulted pages which maps to nullptr.
    ///      This can be avoided by carefully allocating the memory region.
    pub absolute_offset_page_table: bool,
    /// Masks out the first N bits in host pointers from the page table.
    /// The intention behind this is to allow users of Dynarmic to pack attributes in the
    /// same integer and update the pointer attribute pair atomically.
    /// If the configured value is 3, all pointers will be forcefully aligned to 8 bytes.
    pub page_table_pointer_mask_bits: std::os::raw::c_int,
    /// Determines if we should detect memory accesses via page_table that straddle are
    /// misaligned. Accesses that straddle page boundaries will fallback to the relevant
    /// memory callback.
    /// This value should be the required access sizes this applies to ORed together.
    /// To detect any access, use: 8 | 16 | 32 | 64.
    pub detect_misaligned_access_via_page_table: u8,
    /// Determines if the above option only triggers when the misalignment straddles a
    /// page boundary.
    pub only_detect_misalignment_via_page_table_on_page_boundary: bool,
    pub fastmem_pointer: cxx::CxxOptional<usize>,
    /// Determines if instructions that pagefault should cause recompilation of that block
    /// with fastmem disabled.
    /// Recompiled code will use the page_table if this is available, otherwise memory
    /// accesses will hit the memory callbacks.
    pub recompile_on_fastmem_failure: bool,
    /// Determines if we should use the above fastmem_pointer for exclusive reads and
    /// writes. On x64, dynarmic currently relies on x64 cmpxchg semantics which may not
    /// provide fully accurate emulation.
    pub fastmem_exclusive_access: bool,
    /// Determines if exclusive access instructions that pagefault should cause
    /// recompilation of that block with fastmem disabled. Recompiled code will use memory
    /// callbacks.
    pub recompile_on_exclusive_fastmem_failure: bool,
    pub coprocessors: [cxx::CxxSharedPtr<a32::Coprocessor>; 16],
    /// When set to true, UserCallbacks::InstructionSynchronizationBarrierRaised will be
    /// called when an ISB instruction is executed.
    /// When set to false, ISB will be treated as a NOP instruction.
    pub hook_isb: bool,
    /// Hint instructions would cause ExceptionRaised to be called with the appropriate
    /// argument.
    pub hook_hint_instructions: bool,
    /// This option relates to translation. Generally when we run into an unpredictable
    /// instruction the ExceptionRaised callback is called. If this is true, we define
    /// definite behaviour for some unpredictable instructions.
    pub define_unpredictable_behaviour: bool,
    /// HACK:
    /// This tells the translator a wall clock will be used, thus allowing it
    /// to avoid writting certain unnecessary code only needed for cycle timers.
    pub wall_clock_cntpct: bool,
    /// This allows accurately emulating protection fault handlers. If true, we check
    /// for exit after every data memory access by the emulated program.
    pub check_halt_on_memory_access: bool,
    /// This option allows you to disable cycle counting. If this is set to false,
    /// AddTicks and GetTicksRemaining are never called, and no cycle counting is done.
    pub enable_cycle_counting: bool,
    /// This option relates to the CPSR.E flag. Enabling this option disables modification
    /// of CPSR.E by the emulated program, forcing it to 0.
    /// NOTE: Calling Jit::SetCpsr with CPSR.E=1 while this option is enabled may result
    ///      in unusual behavior.
    pub always_little_endian: bool,
    pub code_cache_size: usize,
    /// Internal use only
    pub very_verbose_debugging_output: bool,
}

// A64

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
    pub memory_read_code: unsafe extern "C" fn(&CallbackRef<T>, a64::VAddr) -> cxx::CxxOptional<u32>,
    #[cfg(target_env = "msvc")]
    pub memory_read_code: unsafe extern "C" fn(&CallbackRef<T>, *mut cxx::CxxOptional<u32>, a64::VAddr),

    pub memory_read_8: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> u8,
    pub memory_read_16: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> u16,
    pub memory_read_32: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> u32,
    pub memory_read_64: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> u64,
    pub memory_read_128: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> u128,
    pub memory_write_8: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u8),
    pub memory_write_16: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u16),
    pub memory_write_32: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u32),
    pub memory_write_64: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u64),
    pub memory_write_128: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u128),
    pub memory_write_exclusive_8: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u8, u8) -> bool,
    pub memory_write_exclusive_16: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u16, u16) -> bool,
    pub memory_write_exclusive_32: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u32, u32) -> bool,
    pub memory_write_exclusive_64: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u64, u64) -> bool,
    pub memory_write_exclusive_128: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, u128, u128) -> bool,

    pub is_readonly_memory: extern "C" fn(&CallbackRef<T>, a64::VAddr) -> bool,
    pub interpreter_fallback: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, usize),
    pub call_svc: extern "C" fn(&mut CallbackRef<T>, u32),
    pub exception_raised: extern "C" fn(&mut CallbackRef<T>, a64::VAddr, a64::Exception),
    pub data_cache_operation_raised: extern "C" fn(&mut CallbackRef<T>, a64::DataCacheOperation, a64::VAddr),
    pub instruction_cache_operation_raised: extern "C" fn(&mut CallbackRef<T>, a64::InstructionCacheOperation, a64::VAddr),
    pub instruction_synchronization_barrier_raised: extern "C" fn(&mut CallbackRef<T>),
    pub add_ticks: extern "C" fn(&mut CallbackRef<T>, u64),
    pub get_ticks_remaining: extern "C" fn(&CallbackRef<T>) -> u64,
    pub get_cntpct: extern "C" fn(&CallbackRef<T>) -> u64,
}

#[repr(C)]
pub struct A64Config<T> {
    pub callbacks: *mut CallbackRef<T>,
    pub processor_id: usize,
    pub global_monitor: *mut ExclusiveMonitor,
    /// This selects other optimizations than can't otherwise be disabled by setting other
    /// configuration options. This includes:
    /// - IR optimizations
    /// - Block linking optimizations
    /// - RSB optimizations
    /// This is intended to be used for debugging.
    pub optimizations: OptimizationFlag,
    /// This enables unsafe optimizations that reduce emulation accuracy in favour of speed.
    /// For safety, in order to enable unsafe optimizations you have to set BOTH this flag
    /// AND the appropriate flag bits above.
    /// The prefered and tested mode for this library is with unsafe optimizations disabled.
    pub unsafe_optimizations: bool,
    /// When set to true, UserCallbacks::DxfxataCacheOperationRaised will be called when any
    /// data cache instruction is executed. Notably DC ZVA will not implicitly do anything.
    /// When set to false, UserCallbacks::DataCacheOperationRaised will never be called.
    /// Executing DC ZVA in this mode will result in zeros being written to memory.
    pub hook_data_cache_operations: bool,
    /// When set to true, UserCallbacks::InstructionSynchronizationBarrierRaised will be
    /// called when an ISB instruction is executed.
    /// When set to false, ISB will be treated as a NOP instruction.
    pub hook_isb: bool,
    /// When set to true, UserCallbacks::ExceptionRaised will be called when any hint
    /// instruction is executed.
    pub hook_hint_instructions: bool,
    /// Counter-timer frequency register. The value of the register is not interpreted by
    /// dynarmic.
    pub cntfrq_el0: u32,
    /// CTR_EL0<27:24> is log2 of the cache writeback granule in words.
    /// CTR_EL0<23:20> is log2 of the exclusives reservation granule in words.
    /// CTR_EL0<19:16> is log2 of the smallest data/unified cacheline in words.
    /// CTR_EL0<15:14> is the level 1 instruction cache policy.
    /// CTR_EL0<3:0> is log2 of the smallest instruction cacheline in words.
    pub ctr_el0: u32,
    /// DCZID_EL0<3:0> is log2 of the block size in words
    /// DCZID_EL0<4> is 0 if the DC ZVA instruction is permitted.
    pub dczid_el0: u32,
    /// Pointer to where TPIDRRO_EL0 is stored. This pointer will be inserted into
    /// emitted code.
    pub tpidrro_el0: *mut u64,
    /// Pointer to where TPIDR_EL0 is stored. This pointer will be inserted into
    /// emitted code.
    pub tpidr_el0: *mut u64,
    /// Pointer to the page table which we can use for direct page table access.
    /// If an entry in page_table is null, the relevant memory callback will be called.
    /// If page_table is nullptr, all memory accesses hit the memory callbacks.
    pub page_table: *mut *mut std::ffi::c_void,
    /// Declares how many valid address bits are there in virtual addresses.
    /// Determines the size of page_table. Valid values are between 12 and 64 inclusive.
    /// This is only used if page_table is not nullptr.
    pub page_table_address_space_bits: usize,
    /// Masks out the first N bits in host pointers from the page table.
    /// The intention behind this is to allow users of Dynarmic to pack attributes in the
    /// same integer and update the pointer attribute pair atomically.
    /// If the configured value is 3, all pointers will be forcefully aligned to 8 bytes.
    pub page_table_pointer_mask_bits: std::os::raw::c_int,
    /// Determines what happens if the guest accesses an entry that is off the end of the
    /// page table. If true, Dynarmic will silently mirror page_table's address space. If
    /// false, accessing memory outside of page_table bounds will result in a call to the
    /// relevant memory callback.
    /// This is only used if page_table is not nullptr.
    pub silently_mirror_page_table: bool,
    /// Determines if the pointer in the page_table shall be offseted locally or globally.
    /// 'false' will access page_table[addr >> bits][addr & mask]
    /// 'true'  will access page_table[addr >> bits][addr]
    /// Note: page_table[addr >> bits] will still be checked to verify active pages.
    ///      So there might be wrongly faulted pages which maps to nullptr.
    ///      This can be avoided by carefully allocating the memory region.
    pub absolute_offset_page_table: bool,
    /// Determines if we should detect memory accesses via page_table that straddle are
    /// misaligned. Accesses that straddle page boundaries will fallback to the relevant
    /// memory callback.
    /// This value should be the required access sizes this applies to ORed together.
    /// To detect any access, use: 8 | 16 | 32 | 64 | 128.
    pub detect_misaligned_access_via_page_table: u8,
    /// Determines if the above option only triggers when the misalignment straddles a
    /// page boundary.
    pub only_detect_misalignment_via_page_table_on_page_boundary: bool,
    /// Fastmem Pointer
    /// This should point to the beginning of a 2^page_table_address_space_bits bytes
    /// address space which is in arranged just like what you wish for emulated memory to
    /// be. If the host page faults on an address, the JIT will fallback to calling the
    /// MemoryRead*MemoryWrite* callbacks.
    pub fastmem_pointer: cxx::CxxOptional<usize>,
    /// Determines if instructions that pagefault should cause recompilation of that block
    /// with fastmem disabled.
    /// Recompiled code will use the page_table if this is available, otherwise memory
    /// accesses will hit the memory callbacks.
    pub recompile_on_fastmem_failure: bool,
    /// Declares how many valid address bits are there in virtual addresses.
    /// Determines the size of fastmem arena. Valid values are between 12 and 64 inclusive.
    /// This is only used if fastmem_pointer is set.
    pub fastmem_address_space_bits: usize,
    /// Determines what happens if the guest accesses an entry that is off the end of the
    /// fastmem arena. If true, Dynarmic will silently mirror fastmem's address space. If
    /// false, accessing memory outside of fastmem bounds will result in a call to the
    /// relevant memory callback.
    /// This is only used if fastmem_pointer is set.
    pub silently_mirror_fastmem: bool,
    /// Determines if we should use the above fastmem_pointer for exclusive reads and
    /// writes. On x64, dynarmic currently relies on x64 cmpxchg semantics which may not
    /// provide fully accurate emulation.
    pub fastmem_exclusive_access: bool,
    /// Determines if exclusive access instructions that pagefault should cause
    /// recompilation of that block with fastmem disabled. Recompiled code will use memory
    /// callbacks.
    pub recompile_on_exclusive_fastmem_failure: bool,
    /// This option relates to translation. Generally when we run into an unpredictable
    /// instruction the ExceptionRaised callback is called. If this is true, we define
    /// definite behaviour for some unpredictable instructions.
    pub define_unpredictable_behaviour: bool,
    /// HACK:
    /// This tells the translator a wall clock will be used, thus allowing it
    /// to avoid writting certain unnecessary code only needed for cycle timers.
    pub wall_clock_cntpct: bool,
    /// This allows accurately emulating protection fault handlers. If true, we check
    /// for exit after every data memory access by the emulated program.
    pub check_halt_on_memory_access: bool,
    /// This option allows you to disable cycle counting. If this is set to false,
    /// AddTicks and GetTicksRemaining are never called, and no cycle counting is done.
    pub enable_cycle_counting: bool,
    pub code_cache_size: usize,
    /// Internal use only
    pub very_verbose_debugging_output: bool,
}
