use crate::internal::cpp::A64::{DataCacheOperation, InstructionCacheOperation, Jit as A64Jit};
pub use crate::internal::{cpp::A64::Exception, A64VAddr as VAddr};
use crate::internal::{A64CallbacksVTable, A64Config, CallbackRef, InternalCallbacks, VTABLE_DIFF};
use num_traits::{PrimInt, Unsigned};
use std::mem::MaybeUninit;

/// Callback functions that dynarmic will use to access memory and
/// when the code calls for a higher exception level (e.g. SVC calls)
pub trait Callbacks : Sized {
    /// All reads through this callback are 4-byte aligned.
    /// Memory must be interpreted as little endian.
    fn memory_read_code(cb: CallbackRef<Self>, addr: VAddr) -> Option<u32> {
        Some(Self::memory_read(cb, addr))
    }

    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_impl(cb: CallbackRef<Self>, addr: VAddr) -> crate::CppOptional<u32> {
        Self::memory_read_code(cb, addr).unwrap_or(0).into()
    }
    #[cfg(target_env = "msvc")]
    unsafe extern "C" fn memory_read_code_impl(cb: CallbackRef<Self>, out: *mut crate::CppOptional<u32>, addr: VAddr) {
        unsafe {
            *out = Self::memory_read_code(cb, addr).unwrap_or(0).into();
        }
    }

    extern "C" fn memory_read<T: PrimInt + Unsigned>(cb: CallbackRef<Self>, addr: VAddr) -> T;
    extern "C" fn memory_write<T: PrimInt + Unsigned>(cb: CallbackRef<Self>, addr: VAddr, val: T);
    extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(cb: CallbackRef<Self>, addr: VAddr, val: T, expected: T) -> bool;
    /// If this callback returns true, the JIT will assume MemoryRead* callbacks will always
    /// return the same value at any point in time for this vaddr. The JIT may use this information
    /// in optimizations.
    /// The default implementation will always return false.
    extern "C" fn is_readonly_memory(cb: CallbackRef<Self>, _addr: VAddr) -> bool {
        false
    }
    /// This function is called when dynarmic doesn't have an implementation for the instruction at `pc` and `num` instructions after.
    /// By default, this function will panic.
    extern "C" fn interpreter_fallback(cb: CallbackRef<Self>, pc: VAddr, num: usize) {
        panic!(
            "dynarmic: Unhandled instruction '0x{:X}' for '{}' instructions at '0x{:X}'",
            Self::memory_read::<u32>(cb, pc),
            num,
            pc
        )
    }
    extern "C" fn raised_exception(cb: CallbackRef<Self>, addr: VAddr, exc: Exception) {
        panic!(
            "dynarmic: Unhandled exception '{:?}' at '0x{:X}'",
            exc, addr
        )
    }
    extern "C" fn call_svc(cb: CallbackRef<Self>, swi: u32);
    extern "C" fn data_cache_operation_raised(cb: CallbackRef<Self>, _op: DataCacheOperation, _addr: VAddr) {}
    extern "C" fn instruction_cache_operation_raised(cb: CallbackRef<Self>, _op: InstructionCacheOperation, _addr: VAddr) {}
    extern "C" fn instruction_synchronization_barrier_raised(cb: CallbackRef<Self>) {}
    /// `ticks` amount of ticks have passed
    extern "C" fn add_ticks(cb: CallbackRef<Self>, ticks: u64);
    /// Returns the remaining amount of ticks before the program stops.
    extern "C" fn get_ticks_remaining(cb: CallbackRef<Self>) -> u64;
    /// Get value in the emulated counter-timer physical count register.
    extern "C" fn get_cntpct(cb: CallbackRef<Self>) -> u64;
}

/// A Rust-safe wrapper of Dynarmic's A64 Jit.
/// This type can be constructed with [Config].
pub struct Jit<'a, T: Callbacks> {
    ptr: *mut A64Jit,
    cpp_cb: Box<InternalCallbacks<T>>,
    rust_cb: &'a mut T,
}

impl<'a, T: Callbacks> Drop for Jit<'a, T> {
    fn drop(&mut self) {
        unsafe {
            crate::internal::cpp::delete_a64_jit(self.ptr)
        }
    }
}

impl<'a, T: Callbacks> Jit<'a, T> {
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
        unsafe { crate::internal::cpp::A64::Jit_Run(self.ptr) }
    }

    /// Step the emulated CPU for one instruction.
    /// Cannot be recursively called.
    #[inline]
    pub fn step(&mut self) -> crate::HaltReason {
        unsafe { crate::internal::cpp::A64::Jit_Step(self.ptr) }
    }

    /// Clears the code cache of all compiled code.
    /// Can be called at any time. Halts execution if called within a callback.
    #[inline]
    pub fn clear_cache(&mut self) {
        unsafe { crate::internal::cpp::A64::Jit_ClearCache(self.ptr) }
    }

    /// Reset CPU state to state at startup. Does not clear code cache.
    /// Cannot be called from a callback.
    #[inline]
    pub fn reset(&mut self) {
        unsafe { crate::internal::cpp::A64::Jit_Reset(self.ptr) }
    }

    /// Stops execution during [Jit::run].
    #[inline]
    pub unsafe fn halt(&mut self, hr: crate::HaltReason) {
        unsafe { crate::internal::cpp::A64::Jit_HaltExecution(self.ptr, hr) }
    }

    /// Clears a halt reason from flags.
    #[inline]
    pub unsafe fn clear_halt(&mut self, hr: crate::HaltReason) {
        unsafe { crate::internal::cpp::A64::Jit_ClearHalt(self.ptr, hr) }
    }

    /// Read Stack Pointer
    #[inline]
    pub fn get_sp(&self) -> u64 {
        unsafe { crate::internal::cpp::A64::Jit_GetSP(self.ptr) }
    }

    /// Modify Stack Pointer
    #[inline]
    pub fn set_sp(&mut self, sp: u64) {
        unsafe { crate::internal::cpp::A64::Jit_SetSP(self.ptr, sp) }
    }

    /// Read Program Counter
    #[inline]
    pub fn get_pc(&self) -> u64 {
        unsafe { crate::internal::cpp::A64::Jit_GetPC(self.ptr) }
    }

    /// Modify Program Counter
    #[inline]
    pub fn set_pc(&mut self, pc: u64) {
        unsafe { crate::internal::cpp::A64::Jit_SetPC(self.ptr, pc) }
    }

    /// Read general-purpose register. (GPR)
    #[inline]
    pub fn get_reg(&self, index: usize) -> u64 {
        unsafe { crate::internal::cpp::A64::Jit_GetRegister(self.ptr, index as _) }
    }

    /// Read the low 32-bits of a GPR.
    #[inline]
    pub fn get_wreg(&self, index: usize) -> u32 {
        self.get_reg(index) as u32
    }

    /// Modify general-purpose register. (GPR)
    #[inline]
    pub fn set_reg(&mut self, index: usize, val: u64) {
        unsafe { crate::internal::cpp::A64::Jit_SetRegister(self.ptr, index as _, val) }
    }

    /// Read all general-purpose registers.
    #[inline]
    pub fn get_regs(&self) -> [u64; 31] {
        unsafe {
            // todo: bindgen can't generate std::array (even though it's the same size as a C/Rust one?) so the return value of GetRegisters is just u8..
            let og = crate::internal::cpp::A64::Jit_GetRegisters as unsafe extern "C" fn(*const A64Jit) -> u8;
            let func: unsafe extern "C" fn(*const A64Jit) -> [u64; 31] = std::mem::transmute(og);

            func(self.ptr)
        }
    }

    /// Replace all general-purpose registers.
    #[inline]
    pub fn set_regs(&mut self, regs: &[u64; 31]) {
        unsafe { crate::internal::cpp::A64::Jit_SetRegisters(self.ptr, regs.as_ptr().cast()) }
    }

    /// Read floating point and SIMD register.
    #[inline]
    pub fn get_vector(&self, index: usize) -> u128 {
        unsafe { crate::internal::cpp::A64::Jit_GetVector(self.ptr, index as _) }
    }

    /// Modify floating point/SIMD register. (GPR)
    #[inline]
    pub fn set_vector(&mut self, index: usize, val: u128) {
        unsafe { crate::internal::cpp::A64::Jit_SetVector(self.ptr, index as _, val) }
    }

    /// Read all floating point and SIMD registers.
    #[inline]
    pub fn get_vectors(&self) -> [u128; 32] {
        unsafe {
            // bindgen can't generate std::array (even though it's the same size as a C/Rust one?) so the return value of GetRegisters is just u8
            let og = crate::internal::cpp::A64::Jit_GetVectors as unsafe extern "C" fn(*const A64Jit) -> u8;
            let func: unsafe extern "C" fn(*const A64Jit) -> [u128; 32] = std::mem::transmute(og);

            func(self.ptr)
        }
    }

    /// Replace all general-purpose registers.
    #[inline]
    pub fn set_vectors(&mut self, regs: &[u128; 32]) {
        unsafe { crate::internal::cpp::A64::Jit_SetRegisters(self.ptr, regs.as_ptr().cast()) }
    }

    /// View FPCR
    #[inline]
    pub fn get_fpcr(&self) -> u32 {
        unsafe { crate::internal::cpp::A64::Jit_GetFpcr(self.ptr) }
    }

    /// Modify FPCR
    #[inline]
    pub fn set_fpcr(&mut self, val: u32) {
        unsafe { crate::internal::cpp::A64::Jit_SetFpcr(self.ptr, val) }
    }

    /// View PSTATE
    #[inline]
    pub fn get_pstate(&self) -> u32 {
        unsafe { crate::internal::cpp::A64::Jit_GetPstate(self.ptr) }
    }

    /// Modify FPCR
    #[inline]
    pub fn set_pstate(&mut self, val: u32) {
        unsafe { crate::internal::cpp::A64::Jit_SetPstate(self.ptr, val) }
    }

    /// Clears exclusive states for this core.
    #[inline]
    pub fn clear_exclusive_state(&mut self) {
        unsafe { crate::internal::cpp::A64::Jit_ClearExclusiveState(self.ptr) }
    }

    /// Returns true if Jit::Run was called but hasn't returned yet.
    /// i.e; we're in a callback
    #[inline]
    pub fn is_executing(&self) -> bool {
        unsafe { crate::internal::cpp::A64::Jit_IsExecuting(self.ptr) }
    }

    /// Dumps the disassembly of all compiled code to stdout.
    #[inline]
    pub fn dump_disassembly(&self) {
        unsafe { crate::internal::cpp::A64::Jit_DumpDisassembly(self.ptr) }
    }

    /// Disassemble the instructions following the current pc and return
    /// the resulting instructions as a vector of their string representations.
    #[inline]
    pub fn disassemble(&self) -> crate::CppVector<crate::cpp_string, crate::cpp_allocator> {
        unsafe {
            if cfg!(not(target_env = "msvc")) {
                std::mem::transmute(crate::internal::cpp::A64::Jit_Disassemble(self.ptr)) // safety: compile-time checks verify vector size
            } else {
                // fix function signature to reflect msvc abi
                let og = crate::internal::cpp::A64::Jit_Disassemble as unsafe extern "C" fn(*const A64Jit) -> _;
                let func: unsafe extern "C" fn(*const A64Jit, *mut crate::CppVector<crate::cpp_string, crate::cpp_allocator>) = std::mem::transmute(og);

                let mut vector = MaybeUninit::uninit();
                func(self.ptr, vector.as_mut_ptr());
                vector.assume_init()
            }
        }
    }
}

pub struct Config<'a, T: Callbacks> {
    cb: &'a mut T,
    pub(crate) config: A64Config<T>
}

impl<'a, T: Callbacks> Config<'a, T> {
    pub fn new(cb: &'a mut T) -> Self {
        Self { cb, config: A64Config {
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
        } }
    }
    pub fn build<'cb>(mut self) -> Jit<'a, T> {
        let mut cpp_cb: Box<InternalCallbacks<T>> = Box::new(InternalCallbacks {
            vtable: unsafe { (&Jit::<T>::CALLBACKS as *const A64CallbacksVTable<T> as *const ()).byte_add(VTABLE_DIFF) }, // safety: vtable_diff is ensured by abi-specific code
            ptr: std::ptr::null_mut(),
            __copy: Default::default(),
        });

        self.config.callbacks =cpp_cb.as_mut() as *mut InternalCallbacks<T>;
        cpp_cb.ptr = self.cb as *mut T;
        Jit {
            ptr: unsafe { crate::internal::cpp::new_a64_jit((&mut self.config as *mut A64Config<T>).cast()) }, // todo: what happens to the config memory here?
            cpp_cb,
            rust_cb: self.cb,
        }
    }
}