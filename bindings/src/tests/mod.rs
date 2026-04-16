mod fastmem;

unsafe extern "C" fn unimplemented() {
    unimplemented!();
}

mod a32 {
    use super::unimplemented;
    use crate::a32::{JitBox, UserCallbacks, UserCallbacksVTable, UserConfig, VAddr};
    use crate::internal::cpp_optional;
    use std::collections::BTreeMap;
    use std::pin::Pin;

    // based off dynarmic testenv
    #[repr(C)]
    struct ArmTestEnv {
        base: UserCallbacks, // acts like a derived of UserCallbacks in C++
        ticks_left: u64,
        code_mem_modified_by_guest: bool,
        code_mem: Vec<u32>,
        modified_memory: BTreeMap<u32, u8>,
        interrupts: Vec<String>,
    }
    const _: () = assert!(std::mem::offset_of!(ArmTestEnv, base) == 0);

    #[cfg(itanium_abi)]
    unsafe extern "C" fn memory_read_code(
        data: *mut UserCallbacks,
        addr: VAddr,
    ) -> cpp_optional<u32> {
        let env = unsafe { &*data.cast::<ArmTestEnv>() };

        if env.is_in_codemem(addr) {
            return env.code_mem[(addr as usize) / 4].into();
        }

        0xEAFFFFFE.into()
    }
    #[cfg(msvc_abi)]
    unsafe extern "C" fn memory_read_code(
        data: *mut UserCallbacks,
        out: *mut cpp_optional<u32>,
        addr: VAddr,
    ) {
        let env = unsafe { &*data.cast::<ArmTestEnv>() };

        if env.is_in_codemem(addr) {
            *out = env.code_mem[(addr as usize) / 4].into();
            return;
        }

        *out = 0xEAFFFFFE.into();
    }
    unsafe extern "C" fn memory_read_8(data: *mut UserCallbacks, addr: VAddr) -> u8 {
        let env = unsafe { &*data.cast::<ArmTestEnv>() };

        if env.is_in_codemem(addr) {
            return env.code_mem[(addr as usize) / 4] as u8;
        }
        if env.modified_memory.contains_key(&addr) {
            return env.modified_memory[&addr];
        }
        addr as u8
    }
    unsafe extern "C" fn memory_read_16(data: *mut UserCallbacks, addr: VAddr) -> u16 {
        memory_read_8(data, addr) as u16 | ((memory_read_8(data, addr + 1) as u16) << 8)
    }
    unsafe extern "C" fn memory_read_32(data: *mut UserCallbacks, addr: VAddr) -> u32 {
        memory_read_16(data, addr) as u32 | ((memory_read_16(data, addr + 2) as u32) << 16)
    }
    unsafe extern "C" fn memory_read_64(data: *mut UserCallbacks, addr: VAddr) -> u64 {
        memory_read_32(data, addr) as u64 | ((memory_read_32(data, addr + 4) as u64) << 32)
    }
    unsafe extern "C" fn memory_write_8(data: *mut UserCallbacks, addr: VAddr, value: u8) {
        let env = unsafe { &mut *data.cast::<ArmTestEnv>() };

        if env.is_in_codemem(addr) {
            env.code_mem_modified_by_guest = true;
        }
        env.modified_memory.insert(addr, value);
    }
    unsafe extern "C" fn memory_write_16(data: *mut UserCallbacks, addr: VAddr, value: u16) {
        memory_write_8(data, addr, value as u8);
        memory_write_8(data, addr + 1, (value >> 8) as u8);
    }
    unsafe extern "C" fn memory_write_32(data: *mut UserCallbacks, addr: VAddr, value: u32) {
        memory_write_16(data, addr, value as u16);
        memory_write_16(data, addr + 2, (value >> 16) as u16);
    }
    unsafe extern "C" fn memory_write_64(data: *mut UserCallbacks, addr: VAddr, value: u64) {
        memory_write_32(data, addr, value as u32);
        memory_write_32(data, addr + 4, (value >> 32) as u32);
    }

    unsafe extern "C" fn add_ticks(data: *mut UserCallbacks, ticks: u64) {
        let env = unsafe { &mut *data.cast::<ArmTestEnv>() };

        if ticks > env.ticks_left {
            env.ticks_left = 0;
            return;
        }
        env.ticks_left -= ticks;
    }
    unsafe extern "C" fn get_ticks_remaining(data: *mut UserCallbacks) -> u64 {
        unsafe { std::mem::transmute::<*mut UserCallbacks, &ArmTestEnv>(data).ticks_left }
    }
    static ArmTestEnv_CALLBACKS: UserCallbacksVTable = UserCallbacksVTable::new(
        Some(memory_read_code),
        None,
        None,
        None,
        Some(memory_read_8),
        Some(memory_read_16),
        Some(memory_read_32),
        Some(memory_read_64),
        Some(memory_write_8),
        Some(memory_write_16),
        Some(memory_write_32),
        Some(memory_write_64),
        None,
        None,
        None,
        None,
        None,
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn ()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn ()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn ()) }),
        None,
        Some(add_ticks),
        Some(get_ticks_remaining),
    );
    impl ArmTestEnv {
        fn is_in_codemem(&self, vaddr: u32) -> bool {
            (vaddr as usize) < 4 * self.code_mem.len()
        }
        fn new() -> Pin<Box<Self>> {
            Box::pin(ArmTestEnv {
                base: UserCallbacks::new(&ArmTestEnv_CALLBACKS),
                ticks_left: 0,
                code_mem_modified_by_guest: false,
                code_mem: vec![],
                modified_memory: BTreeMap::new(),
                interrupts: vec![],
            })
        }
    }

    #[test]
    fn a32_add() {
        unsafe {
            let mut env = ArmTestEnv::new();
            let mut jit = JitBox::new(UserConfig::new(&mut env.base, None));

            env.code_mem.push(0xe0810002); // ADD R0, R1, R2
            env.code_mem.push(0xeafffffe); // B .

            let regs: *mut [u32; 16] = jit.Regs() as *mut [u32; 16];

            (*regs)[0] = 0;
            (*regs)[1] = 1;
            (*regs)[2] = 2;

            env.ticks_left = 2;
            jit.Run();

            let regs = jit.Regs() as *const [u32; 16];
            assert_eq!((*regs)[0], 3);
            assert_eq!((*regs)[1], 1);
            assert_eq!((*regs)[2], 2);
            //assert_eq!(jit.Regs(), 4);
        }
    }
}

mod a64 {
    use crate::a64::{Exception, JitBox, UserCallbacks, UserCallbacksVTable, UserConfig, VAddr};
    use crate::internal::cpp_optional;
    use std::collections::BTreeMap;
    use std::pin::Pin;
    use crate::tests::unimplemented;

    // based off dynarmic testenv
    #[repr(C)]
    struct A64TestEnv {
        base: UserCallbacks, // acts like a derived of UserCallbacks in C++
        ticks_left: u64,
        code_mem_modified_by_guest: bool,
        code_mem_start_addr: u64,
        code_mem: Vec<u32>,

        modified_memory: BTreeMap<u64, u8>,
        interrupts: Vec<String>,
    }

    const _: () = assert!(std::mem::offset_of!(A64TestEnv, base) == 0);

    #[cfg(itanium_abi)]
    unsafe extern "C" fn memory_read_code(
        data: *mut UserCallbacks,
        addr: VAddr,
    ) -> cpp_optional<u32> {
        let env = unsafe { &*data.cast::<A64TestEnv>() };

        if !env.is_in_codemem(addr) {
            return 0x14000000.into(); // B .
        }

        let index = (addr - env.code_mem_start_addr) as usize / 4;
        (*env.code_mem.get(index).unwrap()).into()
    }
    #[cfg(msvc_abi)]
    unsafe extern "C" fn memory_read_code(
        data: *mut UserCallbacks,
        out: *mut cpp_optional<u32>,
        addr: VAddr,
    ) {
        let env = unsafe { &*data.cast::<A64TestEnv>() };

        if !env.is_in_codemem(addr) {
            *out = 0x14000000.into(); // B .
            return;
        }

        let index = (addr - env.code_mem_start_addr) as usize / 4;
        *out = (*env.code_mem.get(index).unwrap()).into();
    }

    unsafe extern "C" fn memory_read_8(data: *mut UserCallbacks, addr: VAddr) -> u8 {
        let env = unsafe { &*data.cast::<A64TestEnv>() };

        if env.is_in_codemem(addr) {
            return env.code_mem[(addr - env.code_mem_start_addr) as usize] as u8;
        }

        if env.modified_memory.contains_key(&addr) {
            return *env.modified_memory.get(&addr).unwrap();
        }

        addr as u8
    }
    unsafe extern "C" fn memory_read_16(data: *mut UserCallbacks, addr: VAddr) -> u16 {
        memory_read_8(data, addr) as u16 | ((memory_read_8(data, addr + 1) as u16) << 8)
    }
    unsafe extern "C" fn memory_read_32(data: *mut UserCallbacks, addr: VAddr) -> u32 {
        memory_read_16(data, addr) as u32 | ((memory_read_16(data, addr + 2) as u32) << 16)
    }
    unsafe extern "C" fn memory_read_64(data: *mut UserCallbacks, addr: VAddr) -> u64 {
        memory_read_32(data, addr) as u64 | ((memory_read_32(data, addr + 4) as u64) << 32)
    }
    unsafe extern "C" fn memory_read_128(data: *mut UserCallbacks, addr: VAddr) -> u128 {
        memory_read_64(data, addr) as u128 | ((memory_read_64(data, addr + 8) as u128) << 64)
    }

    unsafe extern "C" fn memory_write_8(data: *mut UserCallbacks, addr: VAddr, value: u8) {
        let env = unsafe { &mut *data.cast::<A64TestEnv>() };

        if env.is_in_codemem(addr) {
            env.code_mem_modified_by_guest = true;
        }

        env.modified_memory.insert(addr, value);
    }
    unsafe extern "C" fn memory_write_16(data: *mut UserCallbacks, addr: VAddr, value: u16) {
        memory_write_8(data, addr, value as u8);
        memory_write_8(data, addr + 1, (value >> 8) as u8);
    }
    unsafe extern "C" fn memory_write_32(data: *mut UserCallbacks, addr: VAddr, value: u32) {
        memory_write_16(data, addr, value as u16);
        memory_write_16(data, addr + 2, (value >> 16) as u16);
    }
    unsafe extern "C" fn memory_write_64(data: *mut UserCallbacks, addr: VAddr, value: u64) {
        memory_write_32(data, addr, value as u32);
        memory_write_32(data, addr + 4, (value >> 32) as u32);
    }
    unsafe extern "C" fn memory_write_128(data: *mut UserCallbacks, addr: VAddr, value: u128) {
        memory_write_64(data, addr, value as u64);
        memory_write_64(data, addr + 8, (value >> 64) as u64);
    }
    unsafe extern "C" fn memory_write_exclusive_8(
        data: *mut UserCallbacks,
        addr: VAddr,
        value: u8,
        expected: u8,
    ) -> bool {
        memory_write_8(data, addr, value);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_16(
        data: *mut UserCallbacks,
        addr: VAddr,
        value: u16,
        expected: u16,
    ) -> bool {
        memory_write_16(data, addr, value);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_32(
        data: *mut UserCallbacks,
        addr: VAddr,
        value: u32,
        expected: u32,
    ) -> bool {
        memory_write_32(data, addr, value);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_64(
        data: *mut UserCallbacks,
        addr: VAddr,
        value: u64,
        expected: u64,
    ) -> bool {
        memory_write_64(data, addr, value);
        true
    }
    unsafe extern "C" fn memory_write_exclusive_128(
        data: *mut UserCallbacks,
        addr: VAddr,
        value: u128,
        expected: u128,
    ) -> bool {
        memory_write_128(data, addr, value);
        true
    }

    unsafe extern "C" fn add_ticks(data: *mut UserCallbacks, ticks: u64) {
        let env = unsafe { &mut *data.cast::<A64TestEnv>() };

        if ticks > env.ticks_left {
            env.ticks_left = 0;
            return;
        }
        env.ticks_left -= ticks;
    }

    unsafe extern "C" fn get_ticks_remaining(data: *mut UserCallbacks) -> u64 {
        let env = unsafe { &*data.cast::<A64TestEnv>() };
        env.ticks_left
    }

    unsafe extern "C" fn get_cntpct(data: *mut UserCallbacks) -> u64 {
        let env = unsafe { &*data.cast::<A64TestEnv>() };
        0x10000000000 - env.ticks_left
    }
    unsafe extern "C" fn is_readonly_memory(data: *mut UserCallbacks, _: VAddr) -> bool {
        false
    }
    unsafe extern "C" fn data_cached_operation_raised(data: *mut UserCallbacks) {}
    unsafe extern "C" fn instruction_cache_operation_raised(data: *mut UserCallbacks) {}
    unsafe extern "C" fn instruction_synchronization_barrier_raised(data: *mut UserCallbacks) {}

    static A64TestEnv_CALLBACKS: UserCallbacksVTable = UserCallbacksVTable::new(
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
        Some(is_readonly_memory),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        Some(unsafe { std::mem::transmute(unimplemented as unsafe extern "C" fn()) }),
        Some(data_cached_operation_raised),
        Some(instruction_cache_operation_raised),
        Some(instruction_synchronization_barrier_raised),
        Some(add_ticks),
        Some(get_ticks_remaining),
        Some(get_cntpct),
    );

    impl A64TestEnv {
        fn is_in_codemem(&self, vaddr: VAddr) -> bool {
            vaddr >= self.code_mem_start_addr
                && vaddr < self.code_mem_start_addr + self.code_mem.len() as u64 * 4
        }
        fn new() -> Pin<Box<Self>> {
            Box::pin(A64TestEnv {
                base: UserCallbacks::new(&A64TestEnv_CALLBACKS),
                ticks_left: 0,
                code_mem_modified_by_guest: false,
                code_mem_start_addr: 0,
                code_mem: vec![],
                modified_memory: BTreeMap::new(),
                interrupts: vec![],
            })
        }
    }

    #[test]
    fn a64_add() {
        unsafe {
            let mut env = A64TestEnv::new();
            let mut jit = JitBox::new(UserConfig::new(&mut env.base));

            env.code_mem.push(0x8b020020); // ADD X0, X1, X2
            env.code_mem.push(0x14000000); // B .

            jit.SetRegister(0, 0);
            jit.SetRegister(1, 1);
            jit.SetRegister(2, 2);

            env.ticks_left = 2;
            jit.Run();

            assert_eq!(jit.GetRegister(0), 3);
            assert_eq!(jit.GetRegister(1), 1);
            assert_eq!(jit.GetRegister(2), 2);
            assert_eq!(jit.GetPC(), 4);
        }
    }
}
