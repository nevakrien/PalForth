use crate::FALSE;
use crate::PalData;
use crate::TRUE;
use crate::buildins::frame_alloc;
use crate::buildins::frame_free;
use crate::buildins::inject;
use crate::buildins::param_drop;
use crate::buildins::pick;
use crate::buildins::push_local;
use crate::buildins::ret;
use crate::buildins::*;
use crate::vm::Code;
use crate::vm::VmEasyMemory;
use core::cell::UnsafeCell;
use core::panic::AssertUnwindSafe;

#[test]
fn round_trip_inject() {
    let mut mem = VmEasyMemory::<32>::new();
    let mut vm = mem.make_vm();

    let code = [
        Code::basic(frame_alloc, 5),
        //inject to the stack
        Code::basic(push_local, 0),
        Code::basic(pick, 1),
        Code::basic(inject, 5 * size_of::<PalData>() as isize),
        Code::basic(param_drop, 1),
        //inject back out
        Code::basic(pick, 1),
        Code::basic(push_local, 0),
        Code::basic(inject, 5 * size_of::<PalData>() as isize),
        Code::basic(param_drop, 1),
        //epilogue
        Code::basic(frame_free, 5),
        Code::basic(ret, 0),
    ];

    let word = Code::word(&code);

    let mut src: [PalData; 5] = [
        PalData { int: 1 },
        PalData { int: 3 },
        PalData { int: 1 },
        PalData { int: 1 },
        PalData { int: -1 },
    ];
    let mut tgt: [PalData; 5] = [PalData { int: 0 }; 5];

    let psrc = &mut src as *mut _;
    let ptgt = &mut tgt as *mut _;
    let data_stack_head = vm.data_stack.head;

    vm.param_stack.push(ptgt).unwrap();
    vm.param_stack.push(psrc).unwrap();

    extern crate std;
    std::println!("src {psrc:?} tgt {ptgt:?} data_stack_top {data_stack_head:?}",);

    unsafe {
        vm.execute_code(&word as *const Code);

        for (s, t) in src.iter().zip(tgt) {
            assert_eq!(s.int, t.int);
        }
    }
}

#[test]
#[cfg(not(feature = "unchecked_underflow"))]
fn stack_underflow_panics() {
    let mut mem = VmEasyMemory::<8>::new();
    let mut vm = mem.make_vm();

    let prog = [Code::basic(param_drop, 1), Code::basic(ret, 0)];
    let word = Code::word(&prog);

    extern crate std;
    let res = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        vm.execute_code(&word as *const Code); // empty stack → should panic
    }));

    assert!(res.is_err(), "param_drop on empty stack must panic");
}

// /* ───────────────────────── CORE OPS (pick / frame / branch) ───────────────────────── */
#[test]
fn core_operations() {
    let mut mem = VmEasyMemory::<32>::new();
    let mut vm = mem.make_vm();

    /* ---- pick 0 (duplicate top) ---- */
    let dup_code = [Code::basic(pick, 0), Code::basic(ret, 0)];
    let dup_word = Code::word(&dup_code);

    let canary = 321usize as *mut _;
    vm.param_stack.push(canary).unwrap();
    unsafe { vm.execute_code(&dup_word as *const Code) };

    assert_eq!(vm.param_stack.pop().unwrap(), canary);
    assert_eq!(vm.param_stack.pop().unwrap(), canary);

    /* ---- branch-if test ---- */
    let maybe_dup_code = [
        Code::basic(branch, 1),
        Code::basic(pick, 0),
        Code::basic(ret, 0),
    ];

    let maybe_dup = Code::word(&maybe_dup_code);

    let mut b = UnsafeCell::new(PalData { bool: TRUE });
    vm.param_stack.push(b.get() as *mut _).unwrap();
    unsafe { vm.execute_code(&maybe_dup as *const Code) };
    assert_eq!(vm.param_stack.write_index(), 0);

    vm.param_stack.push(canary).unwrap();
    *b.get_mut() = PalData { bool: FALSE };
    vm.param_stack.push(b.get() as *mut _).unwrap();
    unsafe { vm.execute_code(&maybe_dup as *const Code) };
    assert_eq!(vm.param_stack.pop().unwrap(), canary);
    assert_eq!(vm.param_stack.pop().unwrap(), canary);
}

/* ───────────────────────── INTEGER ARITHMETICS ───────────────────────── */

#[test]
fn integer_arithmetics() {
    let mut mem = VmEasyMemory::<16>::new();
    let mut vm = mem.make_vm();

    let mut a = UnsafeCell::new(PalData { int: 67 });
    let mut b = UnsafeCell::new(PalData { int: 67 });

    let mut arith_code = UnsafeCell::new([
        Code::basic(pick, 1),
        Code::basic(pick, 1),
        Code::basic(int_add, 0), // placeholder – overwritten each run
        Code::basic(param_drop, 1),
        Code::basic(ret, 0),
    ]);
    let arith_word = Code::word_raw(arith_code.get() as *const _);

    vm.param_stack.push(a.get()).unwrap();
    vm.param_stack.push(b.get()).unwrap();

    macro_rules! run {
        ($builtin:path, $l:expr, $r:expr, $expect:expr) => {{
            *a.get_mut() = PalData { int: $l };
            *b.get_mut() = PalData { int: $r };
            arith_code.get_mut()[2] = Code::basic($builtin, 0);

            unsafe {
                vm.execute_code(&arith_word as *const Code);
                assert_eq!(a.get_mut().int, $expect);
            }
        }};
    }

    run!(int_add, 1, 2, 1 + 2);
    run!(int_sub, 5, 3, 5 - 3);
    run!(int_mul, 6, 7, 6 * 7);
    run!(int_div, 20, 5, 20 / 5);
    run!(int_shl, 3, 2, 3 << 2);
    run!(int_shr, 16, 2, 16 >> 2);
    run!(int_and, 0b1100, 0b1010, 0b1100 & 0b1010);
    run!(int_or, 0b1100, 0b1010, 0b1100 | 0b1010);
    run!(int_xor, 0b1100, 0b1010, 0b1100 ^ 0b1010);
    run!(int_mod, 17, 5, 17 % 5);

    vm.param_stack.pop().unwrap();
    vm.param_stack.pop().unwrap();
}
/* ───────────────────────── BOOL & COMPARISONS ───────────────────────── */

#[test]
fn bool_and_comparisons() {
    let mut mem = VmEasyMemory::<32>::new();
    let mut vm = mem.make_vm();

    /* ---- comparisons (EQ / NEQ / < / > / <= / >=) ---- */
    let res = UnsafeCell::new(FALSE);
    let mut x = UnsafeCell::new(PalData { int: 0 });
    let mut y = UnsafeCell::new(PalData { int: 0 });

    let mut cmp_code = UnsafeCell::new([
        Code::basic(pick, 2),
        Code::basic(pick, 2),
        Code::basic(pick, 2),
        Code::basic(int_eq, 0), // placeholder
        Code::basic(param_drop, 1),
        Code::basic(ret, 0),
    ]);
    let cmp_word = Code::word_raw(cmp_code.get() as *const _);

    vm.param_stack.push(res.get() as *mut _).unwrap();
    vm.param_stack.push(x.get() as *mut _).unwrap();
    vm.param_stack.push(y.get() as *mut _).unwrap();

    macro_rules! cmp {
        ($builtin:path, $l:expr, $r:expr, $ok:expr) => {{
            *x.get_mut() = PalData { int: $l };
            *y.get_mut() = PalData { int: $r };
            (cmp_code.get_mut())[3] = Code::basic($builtin, 0);
            unsafe {
                vm.execute_code(&cmp_word as *const Code);
                assert_eq!(*res.get() == TRUE, $ok);
            }
        }};
    }

    cmp!(int_eq, 5, 5, true);
    cmp!(int_neq, 5, 6, true);
    cmp!(int_smaller, 2, 3, true);
    cmp!(int_bigger, 9, 4, true);
    cmp!(int_le, 4, 4, true);
    cmp!(int_ge, 7, 2, true);

    for _ in 0..3 {
        vm.param_stack.pop().unwrap();
    }

    /* ---- logical AND / OR / XOR ---- */
    let mut lhs = UnsafeCell::new(PalData { bool: FALSE });
    let mut rhs = UnsafeCell::new(PalData { bool: FALSE });

    let mut bool_code = UnsafeCell::new([
        Code::basic(pick, 1),
        Code::basic(pick, 1),
        Code::basic(bool_and, 0), // placeholder
        Code::basic(param_drop, 1),
        Code::basic(ret, 0),
    ]);
    let bool_word = Code::word_raw(bool_code.get() as *const _);

    vm.param_stack.push(lhs.get() as *mut _).unwrap();
    vm.param_stack.push(rhs.get() as *mut _).unwrap();

    macro_rules! logic {
        ($builtin:path, $l:expr, $r:expr, $expect:expr) => {{
            *lhs.get_mut() = PalData { bool: $l };
            *rhs.get_mut() = PalData { bool: $r };
            (bool_code.get_mut())[2] = Code::basic($builtin, 0);
            unsafe {
                vm.execute_code(&bool_word as *const Code);
                assert_eq!((*lhs.get()).bool == TRUE, $expect);
            };
        }};
    }

    logic!(bool_and, TRUE, TRUE, true);
    logic!(bool_or, FALSE, TRUE, true);
    logic!(bool_xor, TRUE, TRUE, false);

    vm.param_stack.pop().unwrap();
    vm.param_stack.pop().unwrap();

    /* ---- unary NOT ---- */
    let mut v = UnsafeCell::new(TRUE);
    let not_code = [
        Code::basic(bool_not, 0),
        Code::basic(param_drop, 1),
        Code::basic(ret, 0),
    ];
    let not_word = Code::word(&not_code);

    vm.param_stack.push(v.get() as *mut _).unwrap();
    unsafe {
        vm.execute_code(&not_word as *const Code);
        assert!(*v.get_mut() == FALSE);
    }
}

#[test]
fn nested_word_dup() {
    use crate::buildins::*;

    let mut mem = VmEasyMemory::<16>::new();
    let mut vm = mem.make_vm();

    /* ---- inner word: DUP (pick 0) ---- */
    let dup_code = [Code::basic(pick, 0), Code::basic(ret, 0)];
    let dup_word = Code::word(&dup_code); // f == None → user-defined

    /* ---- outer word: call the inner DUP ---- */
    let outer_code = [
        dup_word, // nested call
        Code::basic(ret, 0),
    ];
    let outer_word = Code::word(&outer_code);

    /* ---- run it ---- */
    let canary = 321usize as *mut _;
    vm.param_stack.push(canary).unwrap();

    unsafe { vm.execute_code(&outer_word as *const Code) };

    // Stack should now contain two identical pointers
    assert_eq!(vm.param_stack.pop().unwrap(), canary);
    assert_eq!(vm.param_stack.pop().unwrap(), canary);
    assert_eq!(vm.param_stack.write_index(), 0);
}

#[test]
fn if_branch_behavior() {
    let mut mem = VmEasyMemory::<16>::new();
    let mut vm = mem.make_vm();

    let jump_code = [
        Code::basic(pick, 0), // duplicate top
        Code::basic(ret, 0),
    ];

    let target = (&jump_code as *const Code).wrapping_sub(1);

    let cond_code = [Code::basic_raw(_if, target), Code::basic(ret, 0)];
    let cond_word = Code::word(&cond_code);

    let canary = 999usize as *mut _;
    let mut flag = UnsafeCell::new(TRUE);
    vm.param_stack.push(canary).unwrap();
    vm.param_stack.push(flag.get() as *mut _).unwrap();

    unsafe {
        vm.execute_code(&cond_word as *const Code);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
    }

    // reset and test false path
    *flag.get_mut() = FALSE;
    vm.param_stack.push(canary).unwrap();
    vm.param_stack.push(flag.get() as *mut _).unwrap();

    unsafe {
        vm.execute_code(&cond_word as *const Code);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
        assert_eq!(vm.param_stack.write_index(), 0);
    }
}

#[test]
fn call_dyn_executes_target() {
    let canary = 123usize as *mut _;

    let mut mem = VmEasyMemory::<16>::new();
    let mut vm = mem.make_vm();

    let dup_code = [
        Code::basic(pick, 0),
        Code::basic(no_op, 0),
        Code::basic(ret, 0),
    ];
    let dup_word = Code::word(&dup_code);
    let mut dyn_target = PalData { code: &dup_word };

    //====== threaded =========
    let call_code = [Code::basic(call_dyn_threaded, 0), Code::basic(ret, 0)];
    let call_word = Code::word(&call_code);

    vm.param_stack.push(canary).unwrap();
    vm.param_stack.push(&mut dyn_target).unwrap();

    unsafe {
        vm.execute_code(&call_word as *const Code);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
    }

    //====== non threaded =========
    let call_code = [
        Code::basic(call_dyn, 0),
        Code::basic(no_op, 0),
        Code::basic(no_op, 0),
        Code::basic(ret, 0),
    ];
    let call_word = Code::word(&call_code);

    vm.param_stack.push(canary).unwrap();
    vm.param_stack.push(&mut dyn_target).unwrap();

    unsafe {
        vm.execute_code(&call_word as *const Code);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
        assert_eq!(vm.param_stack.pop().unwrap(), canary);
    }
}
