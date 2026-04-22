pub use crate::internal::cpp::A32::{ArchVersion, Coprocessor, Exception, IREmitter, VAddr};
use crate::internal::{cpp::A32::Jit as A32Jit, A32CallbacksVTable, A32Config, CallbackRef, InternalCallbacks, VTABLE_DIFF};

use crate::{CppOptional, OptimizationFlag};
use num_traits::{PrimInt, Unsigned};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

/// Callback functions that dynarmic will use to access memory and
/// when the code calls for a higher exception level (e.g. SVC calls)
pub trait Callbacks : Sized {
    /// All reads through this callback are 4-byte aligned.
    /// Memory must be interpreted as little endian.
    fn memory_read_code(cb: &mut CallbackRef<Self>, addr: VAddr) -> Option<u32> { Some(Self::memory_read(cb, addr)) }

    #[cfg(not(target_env = "msvc"))]
    unsafe extern "C" fn memory_read_code_impl(cb: &mut CallbackRef<Self>, addr: VAddr) -> CppOptional<u32> {
        Self::memory_read_code(cb, addr).unwrap_or(0).into()
    }
    #[cfg(target_env = "msvc")]
    unsafe extern "C" fn memory_read_code_impl(&self, out: *mut CppOptional<u32>, addr: VAddr) {
        unsafe {
            *out = self.memory_read_code(addr).unwrap_or(0).into();
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

    extern "C" fn memory_read<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr) -> T;
    extern "C" fn memory_write<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T);
    extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(cb: &mut CallbackRef<Self>, addr: VAddr, val: T, expected: T) -> bool;

    /// If this callback returns true, the JIT will assume MemoryRead* callbacks will always
    /// return the same value at any point in time for this vaddr. The JIT may use this information
    /// in optimizations.
    /// The default implementation will always return false.
    extern "C" fn is_readonly_memory(_cb: &mut CallbackRef<Self>, _addr: VAddr) -> bool {
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
    extern "C" fn get_ticks_remaining(cb: &mut CallbackRef<Self>) -> u64;
}

/// A Rust-safe wrapper of Dynarmic's A32 Jit.
/// This type can be constructed with [Config].
#[allow(dead_code)]
pub struct Jit<'a, T: Callbacks> {
    ptr: *mut A32Jit,
    cpp_cb: Box<InternalCallbacks<T>>,
    rust_cb: &'a mut T,
}

impl<'a, T: Callbacks> Drop for Jit<'a, T> {
    fn drop(&mut self) {
        unsafe {
            crate::internal::cpp::delete_a32_jit(self.ptr)
        }
    }
}

impl<'a, T: Callbacks> Jit<'a, T> {
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
        unsafe { crate::internal::cpp::A32::Jit_Run(self.ptr) }
    }

    /// Step the emulated CPU for one instruction.
    /// Cannot be recursively called.
    #[inline]
    pub fn step(&mut self) -> crate::HaltReason {
        unsafe { crate::internal::cpp::A32::Jit_Step(self.ptr) }
    }

    /// Clears the code cache of all compiled code.
    /// Can be called at any time. Halts execution if called within a callback.
    #[inline]
    pub fn clear_cache(&mut self) {
        unsafe { crate::internal::cpp::A32::Jit_ClearCache(self.ptr) }
    }

    /// Reset CPU state to state at startup. Does not clear code cache.
    /// Cannot be called from a callback.
    #[inline]
    pub fn reset(&mut self) {
        unsafe { crate::internal::cpp::A32::Jit_Reset(self.ptr) }
    }

    /// Stops execution during [Jit::run].
    #[inline]
    pub fn halt(&mut self, hr: crate::HaltReason) {
        unsafe { crate::internal::cpp::A32::Jit_HaltExecution(self.ptr, hr) }
    }

    /// Clears a halt reason from flags.
    #[inline]
    pub unsafe fn clear_halt(&mut self, hr: crate::HaltReason) {
        unsafe { crate::internal::cpp::A32::Jit_ClearHalt(self.ptr, hr) }
    }

    /// View general-purpose registers.
    #[inline]
    pub fn get_regs(&self) -> &[u32; 16] {
        unsafe { std::slice::from_raw_parts::<u32>(crate::internal::cpp::A32::Jit_Regs(self.ptr).cast(), 16).try_into().unwrap_unchecked() }
    }

    /// Replace general-purpose registers.
    #[inline]
    pub fn set_regs(&mut self, regs: [u32; 16]) {
        unsafe {
            std::ptr::copy_nonoverlapping(regs.as_ptr(), crate::internal::cpp::A32::Jit_Regs(self.ptr).cast(), 16);
        }
    }

    /// Get raw FP/SIMD registers in units of u32.
    #[inline]
    pub fn get_extregs(&self) -> &[u32; 64] {
        unsafe { std::slice::from_raw_parts::<u32>(crate::internal::cpp::A32::Jit_ExtRegs(self.ptr).cast(), 64).try_into().unwrap_unchecked() }
    }

    /// Replace FP/SIMD registers.
    #[inline]
    pub fn set_extregs(&self, regs: [u32; 64]) {
        unsafe {
            std::ptr::copy_nonoverlapping(regs.as_ptr(), crate::internal::cpp::A32::Jit_ExtRegs(self.ptr).cast(), 64);
        }
    }

    #[inline]
    pub fn get_reg(&self, index: usize) -> u32 {
        self.get_regs()[index]
    }

    #[inline]
    pub fn set_reg(&mut self, index: usize, val: u32) {
        unsafe { std::slice::from_raw_parts_mut::<u32>(crate::internal::cpp::A32::Jit_Regs(self.ptr).cast(), 16)[index] = val }
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
        unsafe { crate::internal::cpp::A32::Jit_Cpsr(self.ptr) }
    }

    /// Modify CPSR
    #[inline]
    pub fn set_cpsr(&mut self, val: u32) {
        unsafe { crate::internal::cpp::A32::Jit_SetCpsr(self.ptr, val) }
    }

    /// View FPSCR
    #[inline]
    pub fn get_fpscr(&self) -> u32 {
        unsafe { crate::internal::cpp::A32::Jit_Fpscr(self.ptr) }
    }

    /// Modify FPSCR
    #[inline]
    pub fn set_fpscr(&mut self, val: u32) {
        unsafe { crate::internal::cpp::A32::Jit_SetFpscr(self.ptr, val) }
    }

    /// Clears exclusive states for this core.
    #[inline]
    pub fn clear_exclusive_state(&mut self) {
        unsafe { crate::internal::cpp::A32::Jit_ClearExclusiveState(self.ptr) }
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
        unsafe { crate::internal::cpp::A32::Jit_DumpDisassembly(self.ptr) }
    }

    /// Disassemble the instructions following the current pc and return
    /// the resulting instructions as a vector of their string representations.
    #[inline]
    pub fn disassemble(&self) -> crate::CppVector<crate::cpp_string, crate::cpp_allocator> {
        unsafe {
            if cfg!(not(target_env = "msvc")) {
                std::mem::transmute(crate::internal::cpp::A32::Jit_Disassemble(self.ptr)) // safety: compile-time checks verify vector size
            } else {
                // fix function signature to reflect msvc abi
                let og = crate::internal::cpp::A32::Jit_Disassemble as unsafe extern "C" fn(*const A32Jit) -> _;
                let func: unsafe extern "C" fn(*const A32Jit, *mut crate::CppVector<crate::cpp_string, crate::cpp_allocator>) = std::mem::transmute(og);

                let mut vector = MaybeUninit::uninit();
                func(self.ptr, vector.as_mut_ptr());
                vector.assume_init()
            }
        }
    }
    
    pub fn callbacks(&mut self) -> &mut CallbackRef<T> {
        unsafe {
            std::mem::transmute(self.cpp_cb.as_mut())
        }
    }
}

impl<'a, T: Callbacks> Deref for Jit<'a, T> {
    type Target = CallbackRef<T>;

    // useless
    fn deref(&self) -> &Self::Target {
        unsafe {
            std::mem::transmute(self.cpp_cb.as_ref())
        }
    }
}

impl<'a, T: Callbacks> DerefMut for Jit<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.callbacks()
    }
}

pub struct Config<'a, T: Callbacks> {
    cb: &'a mut T,
    pub(crate) config: A32Config<T>,
}

impl<'a, T: Callbacks> Config<'a, T> {
    pub fn new(cb: &'a mut T) -> Self {
        Self { config: A32Config {
            callbacks: unsafe { std::mem::zeroed() },
            processor_id: 0,
            global_monitor: unsafe { std::mem::zeroed() }, // todo
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
        }, cb }
    }
    pub fn build<'cb>(mut self) -> Jit<'a, T> {
        let mut cpp_cb: Box<InternalCallbacks<T>> = Box::new(InternalCallbacks {
            vtable: unsafe { (&Jit::<T>::CALLBACKS as *const A32CallbacksVTable<T> as *const ()).byte_add(VTABLE_DIFF) }, // safety: vtable_diff is ensured by abi-specific code
            ptr: std::ptr::null_mut(),
        });

        self.config.callbacks = cpp_cb.as_mut();
        cpp_cb.ptr = self.cb as *mut T;
        Jit {
            ptr: unsafe { crate::internal::cpp::new_a32_jit((&mut self.config as *mut A32Config<T>).cast()) }, // todo: what happens to the config memory here?
            cpp_cb,
            rust_cb: self.cb,
        }
    }
}