use crate::internal::{A32CallbacksVTable, A32Config, VTABLE_DIFF};

use crate::{cxx::CxxOptional, CallbackRef, OptimizationFlag};
use num_traits::{PrimInt, Unsigned};
use std::ops::{Deref, DerefMut};

pub type VAddr = u32;

#[repr(i32)]
#[derive(Debug)]
pub enum Exception {
    /// An UndefinedFault occured due to executing instruction with an unallocated encoding
    UndefinedInstruction = 0,
    /// An unpredictable instruction is to be executed. Implementation-defined behaviour should now happen.
    /// This behaviour is up to the user of this library to define.
    UnpredictableInstruction = 1,
    /// A decode error occurred when decoding this instruction. This should never happen.
    DecodeError = 2,
    /// A SEV instruction was executed. The event register of all PEs should be set. (Hint instruction.)
    SendEvent = 3,
    /// A SEVL instruction was executed. The event register of the current PE should be set. (Hint instruction.)
    SendEventLocal = 4,
    /// A WFI instruction was executed. You may now enter a low-power state. (Hint instruction.)
    WaitForInterrupt = 5,
    /// A WFE instruction was executed. You may now enter a low-power state if the event register is clear. (Hint instruction.)
    WaitForEvent = 6,
    /// A YIELD instruction was executed. (Hint instruction.)
    Yield = 7,
    /// A BKPT instruction was executed.
    Breakpoint = 8,
    /// A PLD instruction was executed. (Hint instruction.)
    PreloadData = 9,
    /// A PLDW instruction was executed. (Hint instruction.)
    PreloadDataWithIntentToWrite = 10,
    /// A PLI instruction was executed. (Hint instruction.)
    PreloadInstruction = 11,
    /// Attempted to execute a code block at an address for which MemoryReadCode returned std::nullopt.
    /// (Intended to be used to emulate memory protection faults.)
    NoExecuteFault = 12,
}

#[repr(i32)]
#[derive(Debug)]
pub enum ArchVersion {
    V3 = 0,
    V4 = 1,
    V4T = 2,
    V5TE = 3,
    V6K = 4,
    V6T2 = 5,
    V7 = 6,
    V8 = 7,
}

#[repr(C)]
pub struct IREmitter; // TODO

#[repr(C)]
pub struct Coprocessor; // TODO

#[repr(i32)]
pub enum CoprocReg {
    C0 = 0,
    C1 = 1,
    C2 = 2,
    C3 = 3,
    C4 = 4,
    C5 = 5,
    C6 = 6,
    C7 = 7,
    C8 = 8,
    C9 = 9,
    C10 = 10,
    C11 = 11,
    C12 = 12,
    C13 = 13,
    C14 = 14,
    C15 = 15,
}

/// Callback functions that dynarmic will use to access memory and
/// when the code calls for a higher exception level (e.g. SVC calls)
pub trait Callbacks : Sized {
    /// All reads through this callback are 4-byte aligned.
    /// Memory must be interpreted as little endian.
    fn memory_read_code(cb: &CallbackRef<Self>, addr: VAddr) -> Option<u32> { Some(Self::memory_read(cb, addr)) }

    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_impl(cb: &CallbackRef<Self>, addr: VAddr) -> CxxOptional<u32> {
        Self::memory_read_code(cb, addr).unwrap_or(0).into()
    }
    #[cfg(target_env = "msvc")]
    unsafe extern "C" fn memory_read_code_impl(cb: &CallbackRef<Self>, out: *mut CxxOptional<u32>, addr: VAddr) {
        unsafe {
            *out = Self::memory_read_code(cb, addr).unwrap_or(0).into();
        }
    }

    /// This function is called before the instruction at pc is read.
    /// IR code can be emitted by the callee prior to instruction handling.
    /// By returning true the callee precludes the translation of the instruction;
    /// in such case the callee is responsible for setting the terminal.
    // TODO: IREmitter
    extern "C" fn pre_code_read_hook(_cb: &mut CallbackRef<Self>, _is_thumb: bool, _pc: VAddr, _ir: &IREmitter) -> bool {
        true
    }

    /// This function is called before the instruction at pc is interpreted.
    /// IR code can be emitted by the callee prior to translation of the instruction.
    extern "C" fn pre_code_translation_hook(_cb: &mut CallbackRef<Self>, _is_thumb: bool, _pc: VAddr, _ir: &IREmitter) {}

    extern "C" fn get_ticks_for_code(_cb: &mut CallbackRef<Self>, _is_thumb: bool, _pc: VAddr, _instruct: u32) -> u64 {
        1
    }

    extern "C" fn memory_read<T: PrimInt + Unsigned>(cb: &CallbackRef<Self>, addr: VAddr) -> T;
    extern "C" fn memory_write<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T);
    extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T, expected: T) -> bool;

    /// If this callback returns true, the JIT will assume MemoryRead* callbacks will always
    /// return the same value at any point in time for this vaddr. The JIT may use this information
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
    extern "C" fn instruction_synchronization_barrier_raised(_cb: &mut CallbackRef<Self>) {}
    /// `ticks` amount of ticks have passed
    extern "C" fn add_ticks(cb: &mut CallbackRef<Self>, ticks: u64);
    /// Returns the remaining amount of ticks before the program stops.
    extern "C" fn get_ticks_remaining(cb: &CallbackRef<Self>) -> u64;
}

#[repr(C)]
pub(crate) struct InternalJit {
    is_executing: bool,
    _ptr: u64
}

const _: () = assert!(size_of::<InternalJit>() == 16);

/// A Rust-safe wrapper of Dynarmic's A32 Jit.
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
            crate::cxx::delete_a32_jit(self.ptr)
        }
    }
}

impl<T: Callbacks> Jit<T> {
    const CALLBACKS: A32CallbacksVTable<T> = A32CallbacksVTable {
        #[cfg(not(target_env = "msvc"))]
        offset_to_top: 0,
        #[cfg(not(target_env = "msvc"))]
        typeinfo: unsafe { std::mem::zeroed() },
        memory_read_code: T::memory_read_code_impl,
        pre_code_read_hook: T::pre_code_read_hook,
        pre_code_translation_hook: T::pre_code_translation_hook,
        get_ticks_for_code: T::get_ticks_for_code,
        cpp_destructor: crate::internal::usercallbacks_destructor,
        #[cfg(not(target_env = "msvc"))]
        itanium_destructor: crate::internal::usercallbacks_destructor,
        memory_read_8: T::memory_read::<u8>,
        memory_read_16: T::memory_read::<u16>,
        memory_read_32: T::memory_read::<u32>,
        memory_read_64: T::memory_read::<u64>,
        memory_write_8: T::memory_write::<u8>,
        memory_write_16: T::memory_write::<u16>,
        memory_write_32: T::memory_write::<u32>,
        memory_write_64: T::memory_write::<u64>,
        memory_write_exclusive_8: T::memory_write_exclusive::<u8>,
        memory_write_exclusive_16: T::memory_write_exclusive::<u16>,
        memory_write_exclusive_32: T::memory_write_exclusive::<u32>,
        memory_write_exclusive_64: T::memory_write_exclusive::<u64>,
        is_readonly_memory: T::is_readonly_memory,
        interpreter_fallback: T::interpreter_fallback,
        call_svc: T::call_svc,
        exception_raised: T::raised_exception,
        instruction_synchronization_barrier_raised: T::instruction_synchronization_barrier_raised,
        add_ticks: T::add_ticks,
        get_ticks_remaining: T::get_ticks_remaining,
    };

    /// Runs the emulated CPU.
    /// Cannot be recursively called.
    /// # Safety:
    /// - All instructions and memory addresses inputted must be valid. Invalid addresses/instructions will cause dynarmic exceptions, which panic by default.
    /// - Some ARM coprocessor instructions may be forwarded to [Coprocessor] callbacks; if these are unhandled (e.g. no coprocessors provided), this may result in a C++ exception, making this function inherently unsafe.
    // TODO: make this function safe
    #[inline]
    pub unsafe fn run(&mut self) -> crate::HaltReason {
        unsafe extern "C" {
            pub fn JitA32_Run(this: *mut InternalJit) -> crate::HaltReason;
        }
        unsafe { JitA32_Run(self.ptr) }
    }

    /// Step the emulated CPU for one instruction.
    /// Cannot be recursively called.
    #[inline]
    pub fn step(&mut self) -> crate::HaltReason {
        unsafe extern "C" {
            pub fn JitA32_Step(this: *mut InternalJit) -> crate::HaltReason;
        }
        unsafe { JitA32_Step(self.ptr) }
    }

    /// Clears the code cache of all compiled code.
    /// Can be called at any time. Halts execution if called within a callback.
    #[inline]
    pub fn clear_cache(&mut self) {
        unsafe extern "C" {
            pub fn JitA32_ClearCache(this: *mut InternalJit);
        }
        unsafe { JitA32_ClearCache(self.ptr) }
    }

    /// Invalidate the code cache at a range of addresses.
    /// @param start_address The starting address of the range to invalidate.
    /// @param length The length (in bytes) of the range to invalidate.
    #[inline]
    pub fn invalidate_cache_range(&mut self, start_addr: VAddr, length: usize) {
        unsafe extern "C" {
            pub fn JitA32_InvalidateCacheRange(this: *mut InternalJit, start_address: u32, length: usize);
        }
        unsafe { JitA32_InvalidateCacheRange(self.ptr, start_addr, length) }
    }

    /// Reset CPU state to state at startup. Does not clear code cache.
    /// Cannot be called from a callback.
    #[inline]
    pub fn reset(&mut self) {
        unsafe extern "C" {
            pub fn JitA32_Reset(this: *mut InternalJit);
        }
        unsafe { JitA32_Reset(self.ptr) }
    }

    /// Stops execution during [Jit::run].
    #[inline]
    pub fn halt(&mut self, hr: crate::HaltReason) {
        unsafe extern "C" {
            pub fn JitA32_HaltExecution(this: *mut InternalJit, hr: crate::HaltReason);
        }
        unsafe { JitA32_HaltExecution(self.ptr, hr) }
    }

    /// Clears a halt reason from flags.
    #[inline]
    pub unsafe fn clear_halt(&mut self, hr: crate::HaltReason) {
        unsafe extern "C" {
            pub fn JitA32_ClearHalt(this: *mut InternalJit, hr: crate::HaltReason);
        }
        unsafe { JitA32_ClearHalt(self.ptr, hr) }
    }

    /// View general-purpose registers.
    #[inline]
    pub fn get_regs(&self) -> &[u32; 16] {
        unsafe extern "C" {
            pub fn JitA32_Regs(this: *mut InternalJit) -> *mut u8;
        }
        unsafe { &*(JitA32_Regs(self.ptr).cast::<[u32; 16]>()) }
    }

    /// Replace general-purpose registers.
    #[inline]
    pub fn set_regs(&mut self, regs: [u32; 16]) {
        unsafe extern "C" {
            pub fn JitA32_Regs(this: *mut InternalJit) -> *mut u8;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(regs.as_ptr(), JitA32_Regs(self.ptr).cast(), 16);
        }
    }

    /// Get raw FP/SIMD registers in units of u32.
    #[inline]
    pub fn get_extregs(&self) -> &[u32; 64] {
        unsafe extern "C" {
            pub fn JitA32_ExtRegs(this: *mut InternalJit) -> *mut u8;
        }
        unsafe { &*(JitA32_ExtRegs(self.ptr).cast::<[u32; 64]>()) }
    }

    /// Replace FP/SIMD registers.
    #[inline]
    pub fn set_extregs(&self, regs: [u32; 64]) {
        unsafe extern "C" {
            pub fn JitA32_ExtRegs(this: *mut InternalJit) -> *mut u8;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(regs.as_ptr(), JitA32_ExtRegs(self.ptr).cast(), 64);
        }
    }

    #[inline]
    pub fn get_reg(&self, index: usize) -> u32 {
        self.get_regs()[index]
    }

    #[inline]
    pub fn set_reg(&mut self, index: usize, val: u32) {
        unsafe {
            // SAFETY: get_regs returns a raw reference
            *(self.get_regs() as *const u32 as *mut u32).wrapping_add(index) = val;
        }
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
        unsafe extern "C" {
            pub fn JitA32_Cpsr(this: *const InternalJit) -> u32;
        }
        unsafe { JitA32_Cpsr(self.ptr) }
    }

    /// Modify CPSR
    #[inline]
    pub fn set_cpsr(&mut self, val: u32) {
        unsafe extern "C" {
            pub fn JitA32_SetCpsr(this: *mut InternalJit, value: u32);
        }
        unsafe { JitA32_SetCpsr(self.ptr, val) }
    }

    /// View FPSCR
    #[inline]
    pub fn get_fpscr(&self) -> u32 {
        unsafe extern "C" {
            pub fn JitA32_Fpscr(this: *const InternalJit) -> u32;
        }
        unsafe { JitA32_Fpscr(self.ptr) }
    }

    /// Modify FPSCR
    #[inline]
    pub fn set_fpscr(&mut self, val: u32) {
        unsafe extern "C" {
            pub fn JitA32_SetFpscr(this: *mut InternalJit, value: u32, );
        }
        unsafe { JitA32_SetFpscr(self.ptr, val) }
    }

    /// Clears exclusive states for this core.
    #[inline]
    pub fn clear_exclusive_state(&mut self) {
        unsafe extern "C" {
            pub fn JitA32_ClearExclusiveState(this: *mut InternalJit);
        }
        unsafe { JitA32_ClearExclusiveState(self.ptr) }
    }

    /// Returns true if Jit::Run was called but hasn't returned yet.
    /// i.e; we're in a callback
    #[inline]
    pub fn is_executing(&self) -> bool {
        unsafe { (*self.ptr).is_executing }
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
    config: A32Config<T>,
    cb: Box<T>
}

impl<T: Callbacks> Config<T> {
    pub fn new(cb: T) -> Self {
        Self { config: A32Config {
            callbacks: unsafe { std::mem::zeroed() },
            processor_id: 0,
            global_monitor: unsafe { std::mem::zeroed() }, // todo
            arch_version: ArchVersion::V8,
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
        }, cb: Box::new(cb) }
    }
    pub fn build<'cb>(self) -> Jit<T> {
        let mut cpp_cb: Box<CallbackRef<T>> = Box::new(CallbackRef {
            vtable: unsafe { (&Jit::<T>::CALLBACKS as *const A32CallbacksVTable<T> as *const ()).byte_add(VTABLE_DIFF) }, // SAFETY: vtable_diff is ensured by abi-specific code
            ptr: self.cb.as_ref() as *const _ as *mut _,
        });

        Jit {
            ptr: unsafe { &mut *crate::cxx::new_a32_jit_t(self.config, cpp_cb.as_mut()) },
            cpp_cb,
            rust_cb: self.cb,
        }
    }

    /// Sets the processor id.
    pub fn processor_id(&mut self, id: usize) -> &mut Self {
        self.config.processor_id = id;
        
        self
    }
    
    /// Sets the fastmem pointer and determines whether dynarmic should recompile without fastmem
    /// if a pagefault is reached.
    /// 
    /// # Safety:
    /// - `ptr` must be a valid pointer pointing to a memory space allocated with read/write access.
    /// - The guest code should not attempt to access outside the memory range of `ptr`. Note that dynarmic will not fallback to [memory_read](Callbacks::memory_read)/[memory_write](Callbacks::memory_write)
    /// when exceeding the fastmem address space as it does not know the size.
    /// 
    pub unsafe fn fastmem(&mut self, ptr: *mut std::ffi::c_void, recompile_on_fault: bool) -> &mut Self {
        self.config.fastmem_pointer = unsafe { std::mem::transmute::<_, usize>(ptr).into() };
        self.config.recompile_on_fastmem_failure = recompile_on_fault;
        
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