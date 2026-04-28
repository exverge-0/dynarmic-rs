use crate::internal::{A64CallbacksVTable, A64Config, VTABLE_DIFF};
use crate::CallbackRef;
use num_traits::{PrimInt, Unsigned};
use std::ops::{Deref, DerefMut};

pub type VAddr = u64;

#[repr(i32)]
#[derive(Debug)]
pub enum Exception {
    /// An UndefinedFault occured due to executing instruction with an unallocated encoding
    UnallocatedEncoding = 0,
    /// An UndefinedFault occured due to executing instruction containing a reserved value
    ReservedValue = 1,
    /// An unpredictable instruction is to be executed. Implementation-defined behaviour should now happen.
    /// This behaviour is up to the user of this library to define.
    /// Note: Constraints on unpredictable behaviour are specified in the ARMv8 ARM.
    UnpredictableInstruction = 2,
    /// A WFI instruction was executed. You may now enter a low-power state. (Hint instruction.)
    WaitForInterrupt = 3,
    /// A WFE instruction was executed. You may now enter a low-power state if the event register is clear. (Hint instruction.)
    WaitForEvent = 4,
    /// A SEV instruction was executed. The event register of all PEs should be set. (Hint instruction.)
    SendEvent = 5,
    /// A SEVL instruction was executed. The event register of the current PE should be set. (Hint instruction.)
    SendEventLocal = 6,
    /// A YIELD instruction was executed. (Hint instruction.)
    Yield = 7,
    /// A BRK instruction was executed. (Hint instruction.)
    Breakpoint = 8,
    /// Attempted to execute a code block at an address for which MemoryReadCode returned std::nullopt.
    /// (Intended to be used to emulate memory protection faults.)
    NoExecuteFault = 9,
}

#[repr(i32)]
#[derive(Debug)]
pub enum DataCacheOperation {
    /// DC CISW
    CleanAndInvalidateBySetWay = 0,
    /// DC CIVAC
    CleanAndInvalidateByVAToPoC = 1,
    /// DC CSW
    CleanBySetWay = 2,
    /// DC CVAC
    CleanByVAToPoC = 3,
    /// DC CVAU
    CleanByVAToPoU = 4,
    /// DC CVAP
    CleanByVAToPoP = 5,
    /// DC ISW
    InvalidateBySetWay = 6,
    /// DC IVAC
    InvalidateByVAToPoC = 7,
    /// DC ZVA
    ZeroByVA = 8,
}

#[repr(i32)]
#[derive(Debug)]
pub enum InstructionCacheOperation {
    /// IC IVAU
    InvalidateByVAToPoU = 0,
    /// IC IALLU
    InvalidateAllToPoU = 1,
    /// IC IALLUIS
    InvalidateAllToPoUInnerSharable = 2,
}

/// Callback functions that dynarmic will use to access memory and
/// when the code calls for a higher exception level (e.g. SVC calls)
pub trait Callbacks : Sized {
    /// All reads through this callback are 4-byte aligned.
    /// Memory must be interpreted as little endian.
    fn memory_read_code(cb: &CallbackRef<Self>, addr: VAddr) -> Option<u32> {
        Some(Self::memory_read(cb, addr))
    }

    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_impl(cb: &CallbackRef<Self>, addr: VAddr) -> crate::cxx::CxxOptional<u32> {
        Self::memory_read_code(cb, addr).unwrap_or(0).into()
    }
    #[cfg(target_env = "msvc")]
    unsafe extern "C" fn memory_read_code_impl(cb: &CallbackRef<Self>, out: *mut crate::cxx::CxxOptional<u32>, addr: VAddr) {
        unsafe {
            *out = Self::memory_read_code(cb, addr).unwrap_or(0).into();
        }
    }

    extern "C" fn memory_read<T: PrimInt + Unsigned>(cb: &CallbackRef<Self>, addr: VAddr) -> T;
    extern "C" fn memory_write<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T);
    extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T, expected: T) -> bool;
    /// If this callback returns true, the Jit will assume MemoryRead* callbacks will always
    /// return the same value at any point in time for this vaddr. The Jit may use this information
    /// in optimizations.
    /// The default implementation will always return false.
    extern "C" fn is_readonly_memory(_cb: &CallbackRef<Self>, _addr: VAddr) -> bool {
        false
    }
    /// This function is called when dynarmic doesn't have an implementation for the instruction at `pc` and `num` instructions after.
    /// By default, this function will panic.
    extern "C" fn interpreter_fallback(cb: &mut CallbackRef<Self>, pc: VAddr, num: usize) {
        panic!(
            "dynarmic: Unhandled instruction '0x{:X}' for '{}' instructions at '0x{:X}'",
            Self::memory_read::<u32>(cb, pc),
            num,
            pc
        )
    }
    extern "C" fn raised_exception(_cb: &mut CallbackRef<Self>, addr: VAddr, exc: Exception) {
        panic!(
            "dynarmic: Unhandled exception '{:?}' at '0x{:X}'",
            exc, addr
        )
    }
    extern "C" fn call_svc(cb: &mut CallbackRef<Self>, swi: u32);
    extern "C" fn data_cache_operation_raised(_cb: &mut CallbackRef<Self>, _op: DataCacheOperation, _addr: VAddr) {}
    extern "C" fn instruction_cache_operation_raised(_cb: &mut CallbackRef<Self>, _op: InstructionCacheOperation, _addr: VAddr) {}
    extern "C" fn instruction_synchronization_barrier_raised(_cb: &mut CallbackRef<Self>) {}
    /// `ticks` amount of ticks have passed
    extern "C" fn add_ticks(cb: &mut CallbackRef<Self>, ticks: u64);
    /// Returns the remaining amount of ticks before the program stops.
    extern "C" fn get_ticks_remaining(cb: &CallbackRef<Self>) -> u64;
    /// Get value in the emulated counter-timer physical count register.
    extern "C" fn get_cntpct(cb: &CallbackRef<Self>) -> u64;
}

pub type InternalJit = u64;

/// A Rust-safe wrapper of Dynarmic's A64 Jit.
/// This type can be constructed with [Config].
#[allow(dead_code)]
pub struct Jit<T: Callbacks> {
    ptr: *mut InternalJit,
    cpp_cb: Box<CallbackRef<T>>,
    rust_cb: Box<T>,
}

impl<T: Callbacks> Drop for Jit<T> {
    fn drop(&mut self) {
        unsafe {
            crate::cxx::bindings::delete_a64_jit(self.ptr)
        }
    }
}

impl<T: Callbacks> Jit<T> {
    const CALLBACKS: A64CallbacksVTable<T> = A64CallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        offset_to_top: 0,
        #[cfg(not(target_env = "msvc"))]
        typeinfo: unsafe { std::mem::zeroed() },
        memory_read_code: T::memory_read_code_impl,
        cpp_destructor: crate::internal::usercallbacks_destructor,
        #[cfg(not(target_env = "msvc"))]
        itanium_destructor: crate::internal::usercallbacks_destructor,
        memory_read_8: T::memory_read::<u8>,
        memory_read_16: T::memory_read::<u16>,
        memory_read_32: T::memory_read::<u32>,
        memory_read_64: T::memory_read::<u64>,
        memory_read_128: T::memory_read::<u128>,
        memory_write_8: T::memory_write::<u8>,
        memory_write_16: T::memory_write::<u16>,
        memory_write_32: T::memory_write::<u32>,
        memory_write_64: T::memory_write::<u64>,
        memory_write_128: T::memory_write::<u128>,
        memory_write_exclusive_8: T::memory_write_exclusive::<u8>,
        memory_write_exclusive_16: T::memory_write_exclusive::<u16>,
        memory_write_exclusive_32: T::memory_write_exclusive::<u32>,
        memory_write_exclusive_64: T::memory_write_exclusive::<u64>,
        memory_write_exclusive_128: T::memory_write_exclusive::<u128>,
        is_readonly_memory: T::is_readonly_memory,
        interpreter_fallback: T::interpreter_fallback,
        call_svc: T::call_svc,
        exception_raised: T::raised_exception,
        data_cache_operation_raised: T::data_cache_operation_raised,
        instruction_cache_operation_raised: T::instruction_cache_operation_raised,
        instruction_synchronization_barrier_raised: T::instruction_synchronization_barrier_raised,
        add_ticks: T::add_ticks,
        get_ticks_remaining: T::get_ticks_remaining,
        get_cntpct: T::get_cntpct,
    };

    /// Runs the emulated CPU.
    /// Cannot be recursively called.
    #[inline]
    pub fn run(&mut self) -> crate::HaltReason {
        unsafe extern "C" {
            pub fn JitA64_Run(this: *mut InternalJit) -> crate::HaltReason;
        }
        unsafe { JitA64_Run(self.ptr) }
    }

    /// Step the emulated CPU for one instruction.
    /// Cannot be recursively called.
    #[inline]
    pub fn step(&mut self) -> crate::HaltReason {
        unsafe extern "C" {
            pub fn JitA64_Step(this: *mut InternalJit) -> crate::HaltReason;
        }
        unsafe { JitA64_Step(self.ptr) }
    }

    /// Clears the code cache of all compiled code.
    /// Can be called at any time. Halts execution if called within a callback.
    #[inline]
    pub fn clear_cache(&mut self) {
        unsafe extern "C" {
            pub fn JitA64_ClearCache(this: *mut InternalJit);
        }
        unsafe { JitA64_ClearCache(self.ptr) }
    }

    /// Invalidate the code cache at a range of addresses.
    /// @param start_address The starting address of the range to invalidate.
    /// @param length The length (in bytes) of the range to invalidate.
    #[inline]
    pub fn invalidate_cache_range(&mut self, start_addr: VAddr, length: usize) {
        unsafe extern "C" {
            pub fn JitA64_InvalidateCacheRange(this: *mut InternalJit, start_address: u64, length: usize);
        }
        unsafe { JitA64_InvalidateCacheRange(self.ptr, start_addr, length) }
    }

    /// Reset CPU state to state at startup. Does not clear code cache.
    /// Cannot be called from a callback.
    #[inline]
    pub fn reset(&mut self) {
        unsafe extern "C" {
            pub fn JitA64_Reset(this: *mut InternalJit);
        }
        unsafe { JitA64_Reset(self.ptr) }
    }

    /// Stops execution during [Jit::run].
    #[inline]
    pub unsafe fn halt(&mut self, hr: crate::HaltReason) {
        unsafe extern "C" {
            pub fn JitA64_HaltExecution(this: *mut InternalJit, hr: crate::HaltReason);
        }
        unsafe { JitA64_HaltExecution(self.ptr, hr) }
    }

    /// Clears a halt reason from flags.
    #[inline]
    pub unsafe fn clear_halt(&mut self, hr: crate::HaltReason) {
        unsafe extern "C" {
            pub fn JitA64_ClearHalt(this: *mut InternalJit, hr: crate::HaltReason);
        }
        unsafe { JitA64_ClearHalt(self.ptr, hr) }
    }

    /// Read Stack Pointer
    #[inline]
    pub fn get_sp(&self) -> u64 {
        unsafe extern "C" {
            pub fn JitA64_GetSP(this: *const InternalJit) -> u64;
        }
        unsafe { JitA64_GetSP(self.ptr) }
    }

    /// Modify Stack Pointer
    #[inline]
    pub fn set_sp(&mut self, sp: u64) {
        unsafe extern "C" {
            pub fn JitA64_SetSP(this: *mut InternalJit, value: u64);
        }
        unsafe { JitA64_SetSP(self.ptr, sp) }
    }

    /// Read Program Counter
    #[inline]
    pub fn get_pc(&self) -> u64 {
        unsafe extern "C" {
            pub fn JitA64_GetPC(this: *const InternalJit) -> u64;
        }
        unsafe { JitA64_GetPC(self.ptr) }
    }

    /// Modify Program Counter
    #[inline]
    pub fn set_pc(&mut self, pc: u64) {
        unsafe extern "C" {
            pub fn JitA64_SetPC(this: *mut InternalJit, value: u64);
        }
        unsafe { JitA64_SetPC(self.ptr, pc) }
    }

    /// Read general-purpose register. (GPR)
    #[inline]
    pub fn get_reg(&self, index: usize) -> u64 {
        unsafe extern "C" {
            pub fn JitA64_GetReg(this: *const InternalJit, index: usize) -> u64;
        }
        unsafe { JitA64_GetReg(self.ptr, index as _) }
    }

    /// Read the low 32-bits of a GPR.
    #[inline]
    pub fn get_wreg(&self, index: usize) -> u32 {
        self.get_reg(index) as u32
    }

    /// Modify general-purpose register. (GPR)
    #[inline]
    pub fn set_reg(&mut self, index: usize, val: u64) {
        unsafe extern "C" {
            pub fn JitA64_SetReg(this: *mut InternalJit, index: usize, value: u64);
        }
        unsafe { JitA64_SetReg(self.ptr, index as _, val) }
    }

    /// Read all general-purpose registers.
    #[inline]
    pub fn get_regs(&self) -> [u64; 31] {
        unsafe extern "C" {
            pub fn JitA64_GetRegs(this: *mut InternalJit, out: *mut [u64; 31]) -> *mut u8;
        }
        let mut regs = [0u64; 31];
        unsafe { JitA64_GetRegs(self.ptr, &mut regs as _); }
        regs
    }

    /// Replace all general-purpose registers.
    #[inline]
    pub fn set_regs(&mut self, regs: &[u64; 31]) {
        unsafe extern "C" {
            pub fn JitA64_SetRegs(this: *mut InternalJit, out: *const u8);
        }
        unsafe { JitA64_SetRegs(self.ptr, regs.as_ptr().cast()) }
    }

    /// Read floating point and SIMD register.
    #[inline]
    pub fn get_vector(&self, index: usize) -> u128 {
        unsafe extern "C" {
            pub fn JitA64_GetVector(this: *const InternalJit, index: usize) -> u128; // todo: msvc?
        }
        unsafe { JitA64_GetVector(self.ptr, index as _) }
    }

    /// Modify floating point/SIMD register. (GPR)
    #[inline]
    pub fn set_vector(&mut self, index: usize, val: u128) {
        unsafe extern "C" {
            pub fn JitA64_SetVector(this: *mut InternalJit, index: usize, value: u128);
        }
        unsafe { JitA64_SetVector(self.ptr, index as _, val) }
    }

    /// Read all floating point and SIMD registers.
    #[inline]
    pub fn get_vectors(&self) -> [u128; 32] {
        unsafe extern "C" {
            pub fn JitA64_GetVectors(this: *mut InternalJit, out: *mut [u128; 32]) -> *mut u8;
        }
        let mut regs = [0u128; 32];
        unsafe { JitA64_GetVectors(self.ptr, &mut regs as _); }
        regs
    }

    /// Replace all general-purpose registers.
    #[inline]
    pub fn set_vectors(&mut self, regs: &[u128; 32]) {
        unsafe extern "C" {
            pub fn JitA64_SetVectors(this: *mut InternalJit, value: *const u8);
        }
        unsafe { JitA64_SetVectors(self.ptr, regs.as_ptr().cast()) }
    }

    /// View FPCR
    #[inline]
    pub fn get_fpcr(&self) -> u32 {
        unsafe extern "C" {
            pub fn JitA64_GetFpcr(this: *const InternalJit) -> u32;
        }
        unsafe { JitA64_GetFpcr(self.ptr) }
    }

    /// Modify FPCR
    #[inline]
    pub fn set_fpcr(&mut self, val: u32) {
        unsafe extern "C" {
            pub fn JitA64_SetFpcr(this: *mut InternalJit, value: u32, );
        }
        unsafe { JitA64_SetFpcr(self.ptr, val) }
    }

    /// View FPSR
    #[inline]
    pub fn get_fpsr(&self) -> u32 {
        unsafe extern "C" {
            pub fn JitA64_GetFpsr(this: *const InternalJit) -> u32;
        }
        unsafe { JitA64_GetFpsr(self.ptr) }
    }

    /// Modify FPSR
    #[inline]
    pub fn set_fpsr(&mut self, val: u32) {
        unsafe extern "C" {
            pub fn JitA64_SetFpsr(this: *mut InternalJit, value: u32);
        }
        unsafe { JitA64_SetFpsr(self.ptr, val) }
    }

    /// View PSTATE
    #[inline]
    pub fn get_pstate(&self) -> u32 {
        unsafe extern "C" {
            pub fn JitA64_GetPstate(this: *const InternalJit) -> u32;
        }
        unsafe { JitA64_GetPstate(self.ptr) }
    }

    /// Modify FPCR
    #[inline]
    pub fn set_pstate(&mut self, val: u32) {
        unsafe extern "C" {
            pub fn JitA64_SetPstate(this: *mut InternalJit, value: u32);
        }
        unsafe { JitA64_SetPstate(self.ptr, val) }
    }

    /// Clears exclusive states for this core.
    #[inline]
    pub fn clear_exclusive_state(&mut self) {
        unsafe extern "C" {
            pub fn JitA64_ClearExclusiveState(this: *mut InternalJit);
        }
        unsafe { JitA64_ClearExclusiveState(self.ptr) }
    }

    /// Returns true if Jit::Run was called but hasn't returned yet.
    /// i.e; we're in a callback
    #[inline]
    pub fn is_executing(&self) -> bool {
        unsafe extern "C" {
            pub fn JitA64_IsExecuting(this: *const InternalJit) -> bool;
        }
        unsafe { JitA64_IsExecuting(self.ptr) }
    }

    /// Dumps the disassembly of all compiled code to stdout.
    #[inline]
    pub fn dump_disassembly(&self) {
        unsafe extern "C" {
            pub fn JitA64_DumpDisassembly(this: *const InternalJit);
        }
        unsafe { JitA64_DumpDisassembly(self.ptr) }
    }

    /// Disassemble the instructions following the current pc and return
    /// the resulting instructions as a vector of their string representations.
    #[inline]
    pub fn disassemble(&self) -> crate::cxx::CxxVector<crate::cxx::CxxString> {
        unsafe extern "C" {
            pub fn JitA64_Disassemble(this: *const InternalJit, out: *mut crate::cxx::CxxVector<crate::cxx::CxxString>);
        }
        todo!()
    }
}

impl<T: Callbacks> Deref for Jit<T> {
    type Target = CallbackRef<T>;

    fn deref(&self) -> &Self::Target {
        self.cpp_cb.as_ref()
    }
}

impl<T: Callbacks> DerefMut for Jit<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cpp_cb.as_mut()
    }
}

pub struct Config<T: Callbacks> {
    config: A64Config<T>,
    cb: Box<T>
}

impl<T: Callbacks> Config<T> {
    pub fn new(cb: T) -> Self {
        Self { config: A64Config {
            callbacks: unsafe { std::mem::zeroed() },
            processor_id: 0,
            global_monitor: unsafe { std::mem::zeroed() }, // todo
            optimizations: crate::OptimizationFlag::AllSafe,
            unsafe_optimizations: false,
            hook_data_cache_operations: false,
            hook_isb: false,
            hook_hint_instructions: false,
            cntfrq_el0: 600000000,
            ctr_el0: 0x8444c004,
            dczid_el0: 4,
            tpidrro_el0: std::ptr::null_mut(),
            tpidr_el0: std::ptr::null_mut(),
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
        }, cb: Box::new(cb) }
    }
    pub fn build<'cb>(self) -> Jit<T> {
        let mut cpp_cb: Box<CallbackRef<T>> = Box::new(CallbackRef {
            vtable: unsafe { (&Jit::<T>::CALLBACKS as *const A64CallbacksVTable<T> as *const ()).byte_add(VTABLE_DIFF) }, // SAFETY: vtable_diff is ensured by abi-specific code
            ptr: self.cb.as_ref() as *const _ as *mut _,
        });

        Jit {
            ptr: unsafe { &mut *crate::cxx::new_a64_jit_t(self.config, cpp_cb.as_mut()) },
            cpp_cb,
            rust_cb: self.cb,
        }
    }

    /// Sets the processor id.
    pub fn processor_id(&mut self, id: usize) -> &mut Self {
        self.config.processor_id = id;

        self
    }

    /// Sets the fastmem pointer.
    /// This should point to the beginning of a 2^`address_space_bits` bytes
    /// address space which is in arranged just like what you wish for emulated memory to
    /// be. If the host page faults on an address, the JIT will fallback to calling the
    /// [memory_read](Callbacks::memory_read)/[memory_write](Callbacks::memory_write) callbacks.
    ///
    /// - `ptr` - pointer to start of memory space
    /// - `address_space_bits` - width of memory space in bits
    /// - `recompile_on_fault` - whether dynarmic should recompile without fastmem when reaching a pagefault
    /// - `silently_mirror` - should dynarmic mirror the address spaces when going out of bounds, otherwise use callbacks
    ///
    /// # Safety
    /// - `ptr` must be a valid pointer pointing to a correctly sized memory space allocated with read/write access.
    pub unsafe fn fastmem(&mut self, ptr: *mut std::ffi::c_void, address_space_bits: usize, recompile_on_fault: bool, silently_mirror: bool) -> &mut Self {
        self.config.fastmem_pointer = unsafe { std::mem::transmute::<_, usize>(ptr).into() };
        self.config.page_table_address_space_bits = address_space_bits;
        self.config.recompile_on_fastmem_failure = recompile_on_fault;
        self.config.silently_mirror_fastmem = silently_mirror;

        self
    }

    /// Determines if dynarmic should use the above fastmem_pointer for exclusive reads and
    /// writes and if dynarmic should recompile without fastmem if a pagefault is reached.
    ///
    /// On x64, dynarmic currently relies on x64 cmpxchg semantics which may not provide
    /// fully accurate emulation.
    pub fn fastmem_exclusive(&mut self, exclusive_access: bool, recompile_on_fault: bool) {
        self.config.fastmem_exclusive_access = exclusive_access;
        self.config.recompile_on_exclusive_fastmem_failure = recompile_on_fault;
    }
}