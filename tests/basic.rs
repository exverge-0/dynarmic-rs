mod fastmem;

mod a32 {
    use dynarmic::a32::{Callbacks, VAddr};
    use dynarmic::{CallbackRef, DynarmicA32};
    use std::collections::BTreeMap;

    // based off dynarmic testenv
    struct ArmTestEnv {
        ticks_left: u64,
        code_mem_modified_by_guest: bool,
        code_mem: Vec<u32>,
        modified_memory: BTreeMap<u32, u8>,
        interrupts: Vec<String>,
    }

    impl Callbacks for ArmTestEnv {
        fn memory_read_code(cb: &CallbackRef<Self>, addr: VAddr) -> Option<u32> {
            if cb.is_in_codemem(addr) {
                return Some(cb.code_mem[(addr as usize) / 4]);
            }

            Some(0xEAFFFFFE)
        }

        extern "C" fn memory_read<T>(_cb: &CallbackRef<Self>, _addr: VAddr) -> T {
            unimplemented!()
        }
        extern "C" fn memory_write<T>(_cb: &mut CallbackRef<Self>, _addr: VAddr, _val: T) {
            unimplemented!()
        }
        extern "C" fn memory_write_exclusive<T>(_cb: &mut CallbackRef<Self>, _addr: VAddr, _val: T, _expected: T) -> bool {
            unimplemented!()
        }

        extern "C" fn call_svc(_cb: &mut CallbackRef<Self>, _swi: u32) {
            unimplemented!()
        }

        extern "C" fn add_ticks(cb: &mut CallbackRef<Self>, ticks: u64) {
            if ticks > cb.ticks_left {
                cb.ticks_left = 0;
                return;
            }
            cb.ticks_left -= ticks;
        }

        extern "C" fn get_ticks_remaining(cb: &CallbackRef<Self>) -> u64 {
            cb.ticks_left
        }
    }

    impl ArmTestEnv {
        fn is_in_codemem(&self, vaddr: u32) -> bool {
            (vaddr as usize) < 4 * self.code_mem.len()
        }
        fn new() -> Self {
            ArmTestEnv {
                ticks_left: 0,
                code_mem_modified_by_guest: false,
                code_mem: vec![],
                modified_memory: BTreeMap::new(),
                interrupts: vec![],
            }
        }
    }

    #[test]
    fn a32_add() {
        unsafe {
            let mut env = ArmTestEnv::new();
            env.code_mem.push(0xe0810002); // ADD R0, R1, R2
            env.code_mem.push(0xeafffffe); // B .
            env.ticks_left = 2;

            let mut jit = DynarmicA32::new(env).build();

            jit.set_reg(0, 0);
            jit.set_reg(1, 1);
            jit.set_reg(2, 2);

            jit.run();

            assert_eq!(jit.get_reg(0), 3);
            assert_eq!(jit.get_reg(1), 1);
            assert_eq!(jit.get_reg(2), 2);
            assert_eq!(jit.get_pc(), 4);
        }
    }
}

mod a64 {
    use dynarmic::{CallbackRef, DynarmicA64};
    use std::collections::BTreeMap;
    use dynarmic::a64::{Callbacks, VAddr};

    // based off dynarmic testenv
    struct A64TestEnv {
        ticks_left: u64,
        code_mem_modified_by_guest: bool,
        code_mem_start_addr: u64,
        code_mem: Vec<u32>,

        modified_memory: BTreeMap<u64, u8>,
        interrupts: Vec<String>,
    }

    impl Callbacks for A64TestEnv {
        fn memory_read_code(cb: &CallbackRef<Self>, addr: VAddr) -> Option<u32> {
            if !cb.is_in_codemem(addr) {
                return 0x14000000.into(); // B .
            }

            let index = (addr - cb.code_mem_start_addr) as usize / 4;
            (*cb.code_mem.get(index).unwrap()).into()
        }
        extern "C" fn memory_read<T>(_cb: &CallbackRef<Self>, _addr: VAddr) -> T {
            unimplemented!()
        }
        extern "C" fn memory_write<T>(_cb: &mut CallbackRef<Self>, _addr: VAddr, _val: T) {
            unimplemented!()
        }
        extern "C" fn memory_write_exclusive<T>(_cb: &mut CallbackRef<Self>, _addr: VAddr, _val: T, _expected: T) -> bool {
            unimplemented!()
        }

        extern "C" fn call_svc(_cb: &mut CallbackRef<Self>, _swi: u32) {
            todo!()
        }

        extern "C" fn add_ticks(cb: &mut CallbackRef<Self>, ticks: u64) {
            if ticks > cb.ticks_left {
                cb.ticks_left = 0;
                return;
            }
            cb.ticks_left -= ticks;
        }

        extern "C" fn get_ticks_remaining(cb: &CallbackRef<Self>) -> u64 {
            cb.ticks_left
        }

        extern "C" fn get_cntpct(cb: &CallbackRef<Self>) -> u64 {
            0x10000000000 - cb.ticks_left
        }
    }

    impl A64TestEnv {
        fn is_in_codemem(&self, vaddr: VAddr) -> bool {
            vaddr >= self.code_mem_start_addr
                && vaddr < self.code_mem_start_addr + self.code_mem.len() as u64 * 4
        }
        fn new() -> Self {
            A64TestEnv {
                ticks_left: 0,
                code_mem_modified_by_guest: false,
                code_mem_start_addr: 0,
                code_mem: vec![],
                modified_memory: BTreeMap::new(),
                interrupts: vec![],
            }
        }
    }

    #[test]
    fn a64_add() {
        let mut env = A64TestEnv::new();

        env.code_mem.push(0x8b020020); // ADD X0, X1, X2
        env.code_mem.push(0x14000000); // B .
        env.ticks_left = 2;

        let mut jit = DynarmicA64::new(env).build();

        jit.set_reg(0, 0);
        jit.set_reg(1, 1);
        jit.set_reg(2, 2);

        unsafe { jit.run(); }

        assert_eq!(jit.get_reg(0), 3);
        assert_eq!(jit.get_reg(1), 1);
        assert_eq!(jit.get_reg(2), 2);
        assert_eq!(jit.get_pc(), 4);
    }
}
