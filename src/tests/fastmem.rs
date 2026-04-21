mod a32 {
    use crate::a32::{Callbacks, Config, VAddr};
    use num_traits::{PrimInt, Unsigned};
    use std::cmp::Ordering;
    use crate::internal::CallbackRef;

    struct ArmFastmemTestEnv {
        ticks_left: u64,
        backing_memory: *mut u8,
    }

    impl ArmFastmemTestEnv {
        fn new(backing_memory: *mut u8) -> Self {
            Self {
                ticks_left: 0,
                backing_memory,
            }
        }
    }

    impl Callbacks for ArmFastmemTestEnv {
        extern "C" fn memory_read<T: PrimInt + Unsigned>(cb: CallbackRef<Self>, addr: VAddr) -> T {
            unsafe {
                cb.backing_memory
                    .wrapping_add(addr as usize)
                    .cast::<T>()
                    .read()
            }
        }

        extern "C" fn memory_write<T: PrimInt + Unsigned>(cb: CallbackRef<Self>, addr: VAddr, val: T) {
            unsafe {
                cb.backing_memory
                    .wrapping_add(addr as usize)
                    .cast::<T>()
                    .copy_from_nonoverlapping(&val, 1);
            }
        }

        extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(
            cb: CallbackRef<Self>,
            addr: VAddr,
            val: T,
            expected: T,
        ) -> bool {
            Self::memory_write(cb, addr, val);
            true
        }

        extern "C" fn call_svc(cb: CallbackRef<Self>, swi: u32) {
            unimplemented!()
        }

        extern "C" fn add_ticks(mut cb: CallbackRef<Self>, ticks: u64) {
            if ticks > cb.ticks_left {
                cb.ticks_left = 0;
                return;
            }
            cb.ticks_left -= ticks;
        }

        extern "C" fn get_ticks_remaining(cb: CallbackRef<Self>) -> u64 {
            cb.ticks_left
        }
    }

    //#[test]
    fn a32_fastmem() {
        unsafe {
            static address_width: usize = 12;
            static memory_size: usize = 1 << address_width; // 4K
            static page_size: usize = 4 * 1024;
            static buffer_size: usize = 2 * page_size; // buffer size is 2x for alignment?
            let mut buffer = vec![0u8; buffer_size];

            let mut ptr = buffer.as_mut_ptr();
            ptr = ptr.add(ptr.align_offset(page_size));

            let mut env = ArmFastmemTestEnv::new(ptr);
            env.ticks_left = 3;
            let mut jit_config = Config::new(&mut env);
            let config = &mut jit_config.config; // TODO: make private

            config.fastmem_pointer = (ptr as usize).into();
            config.recompile_on_fastmem_failure = false;
            config.processor_id = 0;

            let mut jit = jit_config.build();
            std::ptr::copy_nonoverlapping(
                std::ffi::CString::new("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
                    .unwrap()
                    .as_ptr()
                    .cast(),
                ptr.add(0x100),
                57,
            );

            *ptr.add(0).cast::<u32>() = 0xE5904000; // LDR R4, [R0]
            *ptr.add(4).cast::<u32>() = 0xE5814000; // STR R4, [R1]
            *ptr.add(8).cast::<u32>() = 0xEAFFFFFE; // B .
            jit.set_reg(0, 0x100);
            jit.set_reg(1, 0x1F0);

            jit.set_pc(0);
            jit.set_cpsr(0x000001d0); // User-mode

            jit.run();

            assert_eq!(
                std::slice::from_raw_parts(ptr.add(0x100), 4)
                    .cmp(std::slice::from_raw_parts(ptr.add(0x1F0), 4)),
                Ordering::Equal
            );
        }
    }
}
mod a64 {
    use crate::a64::{Callbacks, Config, VAddr};
    use num_traits::{PrimInt, Unsigned};
    use std::cmp::Ordering;
    use crate::internal::CallbackRef;

    #[repr(C)]
    struct A64FastmemTestEnv {
        ticks_left: u64,
        backing_memory: *mut u8,
    }

    impl A64FastmemTestEnv {
        fn new(backing_memory: *mut u8) -> Self {
            Self {
                ticks_left: 0,
                backing_memory,
            }
        }
    }

    impl Callbacks for A64FastmemTestEnv {
        extern "C" fn memory_read<T>(cb: CallbackRef<Self>, vaddr: VAddr) -> T {
            unsafe {
                cb.backing_memory
                    .wrapping_add(vaddr as usize)
                    .cast::<T>()
                    .read()
            }
        }
        extern "C" fn memory_write<T>(cb: CallbackRef<Self>, vaddr: VAddr, val: T) {
            unsafe {
                cb.backing_memory
                    .wrapping_add(vaddr as usize)
                    .cast::<T>()
                    .copy_from_nonoverlapping(&val, 1);
            }
        }
        extern "C" fn memory_write_exclusive<T: PrimInt + Unsigned>(
            cb: CallbackRef<Self>,
            addr: VAddr,
            val: T,
            expected: T,
        ) -> bool {
            Self::memory_write(cb, addr, val);
            true
        }

        extern "C" fn call_svc(cb: CallbackRef<Self>, swi: u32) {
            unimplemented!()
        }

        extern "C" fn add_ticks(mut cb: CallbackRef<Self>, ticks: u64) {
            if ticks > cb.ticks_left {
                cb.ticks_left = 0;
                return;
            }
            cb.ticks_left -= ticks;
        }
        extern "C" fn get_ticks_remaining(cb: CallbackRef<Self>) -> u64 {
            cb.ticks_left
        }
        extern "C" fn get_cntpct(cb: CallbackRef<Self>) -> u64 {
            0x10000000000 - cb.ticks_left
        }
    }

    #[test]
    fn a64_fastmem() {
        unsafe {
            static address_width: usize = 12;
            static memory_size: usize = 1 << address_width; // 4K
            static page_size: usize = 4 * 1024;
            static buffer_size: usize = 2 * page_size; // buffer size is 2x for alignment?
            let mut buffer = vec![0u8; buffer_size];

            let mut ptr = buffer.as_mut_ptr();
            ptr = ptr.add(ptr.align_offset(page_size));

            let mut env = A64FastmemTestEnv::new(ptr);
            env.ticks_left = 5;
            let mut jit_config = Config::new(&mut env);
            let config = &mut jit_config.config; // TODO: make private
            config.fastmem_pointer = (ptr as usize).into();
            config.fastmem_address_space_bits = address_width;
            config.recompile_on_fastmem_failure = false;
            config.silently_mirror_fastmem = true;
            config.processor_id = 0;

            let mut jit = jit_config.build();
            std::ptr::copy_nonoverlapping(
                std::ffi::CString::new("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
                    .unwrap()
                    .as_ptr()
                    .cast(),
                ptr.add(0x100),
                57,
            );

            *ptr.add(0).cast::<u32>() = 0xA9401404; // LDP x4, x5, [x0]
            *ptr.add(4).cast::<u32>() = 0xF9400046; // LDR x6, [x2]
            *ptr.add(8).cast::<u32>() = 0xA9001424; // STP x4, x5, [x1]
            *ptr.add(12).cast::<u32>() = 0xF9000066; // STR x6, [x3]
            *ptr.add(16).cast::<u32>() = 0x14000000; // B .
            jit.set_reg(0, 0x100);
            jit.set_reg(1, 0x1F0);
            jit.set_reg(2, 0x10F);
            jit.set_reg(3, 0x1FF);

            jit.set_pc(0);
            jit.set_sp(memory_size as u64 - 1u64);
            jit.set_fpcr(0x03480000);
            jit.set_pstate(0x30000000);

            jit.run();

            assert_eq!(
                std::slice::from_raw_parts(ptr.add(0x100), 23)
                    .cmp(std::slice::from_raw_parts(ptr.add(0x1F0), 23)),
                Ordering::Equal
            );
        }
    }
}