mod a64 {
    use crate::a64::{Jit, UserCallbacks, UserCallbacksVTable, UserConfig, VAddr};
    use crate::internal::cpp_optional;
    use crate::tests::unimplemented;
    use std::cmp::Ordering;

    #[repr(C)]
    struct A64FastmemTestEnv {
        base: UserCallbacks, // acts like a derived of UserCallbacks in C++ w/o typeinfo
        ticks_left: u64,
        backing_memory: *mut u8,
    }

    const _: () = assert!(std::mem::offset_of!(A64FastmemTestEnv, base) == 0);

    impl A64FastmemTestEnv {
        fn new(backing_memory: *mut u8) -> Self {
            Self {
                base: UserCallbacks::new(&A64FastmemTestEnv_CALLBACKS),
                ticks_left: 0,
                backing_memory
            }
        }
        unsafe fn read<T>(&self, vaddr: VAddr) -> T where T: Copy { // requiring T implement Copy is probably more accurate here
            unsafe {
                self.backing_memory.wrapping_add(vaddr as usize).cast::<T>().read()
            }
        }
        unsafe fn write<T>(&mut self, vaddr: VAddr, val: T) where T: Copy {
            unsafe {
                self.backing_memory.wrapping_add(vaddr as usize).cast::<T>().copy_from_nonoverlapping(&val, 1);
            }
        }
    }

    #[cfg(itanium_abi)]
    unsafe extern "C" fn memory_read_code(env: *mut UserCallbacks, vaddr: VAddr) -> cpp_optional<u32> {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read::<u32>(vaddr).into()
    }

    #[cfg(msvc_abi)]
    unsafe extern "C" fn memory_read_code(env: *mut UserCallbacks, out: *mut cpp_optional<u32>, vaddr: VAddr) {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        *out = env.read::<u32>(vaddr).into()
    }

    unsafe extern "C" fn memory_read_8(env: *mut UserCallbacks, vaddr: VAddr) -> u8 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read(vaddr)
    }
    unsafe extern "C" fn memory_read_16(env: *mut UserCallbacks, vaddr: VAddr) -> u16 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read(vaddr)
    }
    unsafe extern "C" fn memory_read_32(env: *mut UserCallbacks, vaddr: VAddr) -> u32 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read(vaddr)
    }
    unsafe extern "C" fn memory_read_64(env: *mut UserCallbacks, vaddr: VAddr) -> u64 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read(vaddr)
    }
    unsafe extern "C" fn memory_read_128(env: *mut UserCallbacks, vaddr: VAddr) -> u128 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.read(vaddr)
    }

    unsafe extern "C" fn memory_write_8(env: *mut UserCallbacks, vaddr: VAddr, val: u8) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        env.write(vaddr, val);
    }
    unsafe extern "C" fn memory_write_16(env: *mut UserCallbacks, vaddr: VAddr, val: u16) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        env.write(vaddr, val);
    }
    unsafe extern "C" fn memory_write_32(env: *mut UserCallbacks, vaddr: VAddr, val: u32) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        env.write(vaddr, val);
    }
    unsafe extern "C" fn memory_write_64(env: *mut UserCallbacks, vaddr: VAddr, val: u64) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        env.write(vaddr, val);
    }
    unsafe extern "C" fn memory_write_128(env: *mut UserCallbacks, vaddr: VAddr, val: u128) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        env.write(vaddr, val);
    }

    unsafe extern "C" fn memory_write_exclusive_8(env: *mut UserCallbacks, vaddr: VAddr, val: u8, expected: u8) -> bool {
        memory_write_8(env, vaddr, val);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_16(env: *mut UserCallbacks, vaddr: VAddr, val: u16, expected: u16) -> bool {
        memory_write_16(env, vaddr, val);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_32(env: *mut UserCallbacks, vaddr: VAddr, val: u32, expected: u32) -> bool {
        memory_write_32(env, vaddr, val);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_64(env: *mut UserCallbacks, vaddr: VAddr, val: u64, expected: u64) -> bool {
        memory_write_64(env, vaddr, val);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_128(env: *mut UserCallbacks, vaddr: VAddr, val: u128, expected: u128) -> bool {
        memory_write_128(env, vaddr, val);
        true
    }

    unsafe extern "C" fn add_ticks(env: *mut UserCallbacks, ticks: u64) {
        let env = unsafe { &mut *env.cast::<A64FastmemTestEnv>() };
        if ticks > env.ticks_left {
            env.ticks_left = 0;
            return;
        }
        env.ticks_left -= ticks;
    }
    unsafe extern "C" fn get_ticks_remaining(env: *mut UserCallbacks) -> u64 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        env.ticks_left
    }
    unsafe extern "C" fn get_cntpct(env: *mut UserCallbacks) -> u64 {
        let env = unsafe { &*env.cast::<A64FastmemTestEnv>() };
        0x10000000000 - env.ticks_left
    }

    static A64FastmemTestEnv_CALLBACKS: UserCallbacksVTable = UserCallbacksVTable::new(
        Some(memory_read_code),
        Some(memory_read_8),
        Some(memory_read_16),
        Some(memory_read_32),
        Some(memory_read_64),
        Some(memory_read_128),
        Some(memory_write_8),
        Some(memory_write_16),
        Some(memory_write_32),
        Some(memory_write_64),
        Some(memory_write_128),
        Some(memory_write_exclusive_8),
        Some(memory_write_exclusive_16),
        Some(memory_write_exclusive_32),
        Some(memory_write_exclusive_64),
        Some(memory_write_exclusive_128),
        None,
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        None,
        None,
        None,
        Some(add_ticks),
        Some(get_ticks_remaining),
        Some(get_cntpct),
    );

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
            let mut config = UserConfig::new(&mut env.base);
            config.fastmem_pointer = (ptr as usize).into();
            config.fastmem_address_space_bits = address_width;
            config.recompile_on_fastmem_failure = false;
            config.silently_mirror_fastmem = true;
            config.processor_id = 0;

            let mut jit = Jit::new(config);
            std::ptr::copy_nonoverlapping(std::ffi::CString::new("Lorem ipsum dolor sit amet, consectetur adipiscing elit.").unwrap().as_ptr().cast(), ptr.add(0x100), 57);

            *ptr.add(0).cast::<u32>() = 0xA9401404;  // LDP x4, x5, [x0]
            *ptr.add(4).cast::<u32>() = 0xF9400046;  // LDR x6, [x2]
            *ptr.add(8).cast::<u32>() = 0xA9001424;  // STP x4, x5, [x1]
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
            env.ticks_left = 5;

            jit.run();

            assert_eq!(std::slice::from_raw_parts(ptr.add(0x100), 23).cmp(std::slice::from_raw_parts(ptr.add(0x1F0), 23)), Ordering::Equal);
        }
    }
}