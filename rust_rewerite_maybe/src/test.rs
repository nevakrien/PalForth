use std::cell::UnsafeCell;
use crate::TRUE;
use crate::FALSE;
use crate::PalBool;
use crate::buildins::*;
use std::panic::AssertUnwindSafe;
use std::panic;
use crate::PalData;
use crate::buildins::frame_alloc;
use crate::buildins::pick;
use crate::buildins::push_local;
use crate::buildins::inject;
use crate::buildins::param_drop;
use crate::buildins::frame_free;
use crate::buildins::ret;
use crate::vm::Code;
use crate::stack::StackRef;
use crate::vm::Vm;
use std::mem::MaybeUninit;


#[test]
fn round_trip_inject() {
    let mut data = [const { MaybeUninit::uninit() };32];
    let mut params = [MaybeUninit::uninit();32];
    let mut vm = Vm{
        param_stack:StackRef::from_slice(&mut params),
        data_stack:StackRef::from_slice(&mut data),
    };

    let code = [

        Code::basic(frame_alloc,5),
        
        //inject to the stack
        Code::basic(push_local,0),
        Code::basic(pick,1),
        Code::basic(inject,5*size_of::<PalData>() as isize),
        Code::basic(param_drop,1),

        //inject back out
        Code::basic(pick,1),
        Code::basic(push_local,0),
        Code::basic(inject,5*size_of::<PalData>() as isize),
        Code::basic(param_drop,1),

        //epilogue
        Code::basic(frame_free,5),
        Code::basic(ret,0),
    ];

    let word = Code::word(&code);

    let mut src:[PalData;5] = [PalData{int:1},PalData{int:3},PalData{int:1},PalData{int:1},PalData{int:-1}];
    let mut tgt:[PalData;5] = [PalData{int:0};5];

    let psrc = &mut src as *mut _;
    let ptgt = &mut tgt as *mut _;
    let data_stack_head = vm.data_stack.head;

    vm.param_stack.push(ptgt).unwrap();
    vm.param_stack.push(psrc).unwrap();

    println!("src {psrc:?} tgt {ptgt:?} data_stack_top {data_stack_head:?}",);

    unsafe{
        vm.excute_code(&word as *const Code);

        for (s,t) in src.iter().zip(tgt){
            assert_eq!(s.int,t.int);
        }
    }
}

#[test]
#[cfg(not(feature = "unchecked_underflow"))]   
fn stack_underflow_panics() {
    let mut data = [MaybeUninit::uninit(); 8];
    let mut params = [MaybeUninit::uninit(); 8];

    let mut vm = Vm {
        data_stack: StackRef::from_slice(&mut data),
        param_stack: StackRef::from_slice(&mut params),
    };

    let prog = [Code::basic(param_drop, 1), Code::basic(ret, 0)];
    let word = Code::word(&prog);

    let res = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        vm.excute_code(&word as *const Code); // empty stack → should panic
    }));

    assert!(res.is_err(), "param_drop on empty stack must panic");
}

// /* ───────────────────────── CORE OPS (pick / frame / branch) ───────────────────────── */

// #[test]
// fn core_operations() {
//     const CAP: usize = 32;
//     let mut data  = [MaybeUninit::uninit(); CAP];
//     let mut params = [MaybeUninit::uninit(); CAP];

//     let mut vm = Vm {
//         data_stack: StackRef::from_slice(&mut data),
//         param_stack: StackRef::from_slice(&mut params),
//     };

//     /* ---- pick 0 (duplicate top) ---- */
//     let dup_code = [Code::basic(pick, 0), Code::basic(ret, 0)];
//     let dup_word = Code::word(&dup_code);

//     let canary = 321usize as *mut _;
//     vm.param_stack.push(canary).unwrap();
//     unsafe { vm.excute_code(&dup_word as *const Code) };

//     assert_eq!(vm.param_stack.pop().unwrap(), canary);
//     assert_eq!(vm.param_stack.pop().unwrap(), canary);


//     /* ---- branch-if test ---- */
//     let maybe_dup_code = [
//         Code::basic(branch, 1),
//         Code::basic(pick, 0),
//         Code::basic(ret, 0),
//     ];

//     let maybe_dup = Code::word(&maybe_dup_code);

//     let mut b = UnsafeCell::new(TRUE);
//     vm.param_stack.push(b.get() as *mut _).unwrap();
//     unsafe { vm.excute_code(&maybe_dup as *const Code) };
//     assert_eq!(vm.param_stack.write_index(), 0);

//     vm.param_stack.push(canary).unwrap();
//     *b.get_mut() = TRUE;
//     vm.param_stack.push(b.get() as *mut _).unwrap();
//     unsafe { vm.excute_code(&maybe_dup as *const Code) };
//     assert_eq!(vm.param_stack.pop().unwrap(), canary);
//     assert_eq!(vm.param_stack.pop().unwrap(), canary);
// }

/* ───────────────────────── INTEGER ARITHMETICS ───────────────────────── */

#[test]
fn integer_arithmetics() {
    let mut data = [MaybeUninit::uninit(); 16];
    let mut params = [MaybeUninit::uninit(); 16];

    let mut vm = Vm {
        data_stack: StackRef::from_slice(&mut data),
        param_stack: StackRef::from_slice(&mut params),
    };

    let mut a = UnsafeCell::new(PalData{int:67});
    let mut b = UnsafeCell::new(PalData{int:67});

    let mut arith_code = UnsafeCell::new([
        Code::basic(pick, 1),
        Code::basic(pick, 1),
        Code::basic(int_add, 0),   // placeholder – overwritten each run
        Code::basic(param_drop, 1),
        Code::basic(ret, 0),
    ]);
    let arith_word = Code::Derived(arith_code.get() as *const _);

    vm.param_stack.push(a.get()).unwrap();
    vm.param_stack.push(b.get()).unwrap();

    macro_rules! run {
        ($builtin:path, $l:expr, $r:expr, $expect:expr) => {{
            *a.get_mut() = PalData{int:$l};
            *b.get_mut() = PalData{int:$r};
            arith_code.get_mut()[2] = Code::basic($builtin, 0);
            
            unsafe { 
                vm.excute_code(&arith_word as *const Code); 
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
    run!(int_or,  0b1100, 0b1010, 0b1100 | 0b1010);
    run!(int_xor, 0b1100, 0b1010, 0b1100 ^ 0b1010);
    run!(int_mod, 17, 5, 17 % 5);

    vm.param_stack.pop().unwrap();
    vm.param_stack.pop().unwrap();
}
/* ───────────────────────── BOOL & COMPARISONS ───────────────────────── */

// #[test]
// fn bool_and_comparisons() {
//     let mut data  = [MaybeUninit::uninit(); 16];
//     let mut params = [MaybeUninit::uninit(); 16];

//     let mut vm = Vm {
//         data_stack: StackRef::from_slice(&mut data),
//         param_stack: StackRef::from_slice(&mut params),
//     };

//     /* ---- comparisons (EQ / NEQ / < / > / <= / >=) ---- */
//     let mut res = UnsafeCell::new(FALSE);
//     let mut x   = UnsafeCell::new(PalData { int: 0 });
//     let mut y   = UnsafeCell::new(PalData { int: 0 });

//     let mut cmp_code = [
//         Code::basic(pick, 2),
//         Code::basic(pick, 2),
//         Code::basic(pick, 2),
//         Code::basic(int_eq, 0), // placeholder
//         Code::basic(param_drop, 1),
//         Code::basic(ret, 0),
//     ];
//     let cmp_word = Code::word(&cmp_code);

//     vm.param_stack.push(res.get() as *mut _).unwrap();
//     vm.param_stack.push(x.get()   as *mut _).unwrap();
//     vm.param_stack.push(y.get()   as *mut _).unwrap();

//     macro_rules! cmp {
//         ($builtin:path, $l:expr, $r:expr, $ok:expr) => {{
//             *x.get_mut() = PalData { int: $l };
//             *y.get_mut() = PalData { int: $r };
//             cmp_code[3]  = Code::basic($builtin, 0);
//             unsafe { 
//                 vm.excute_code(&cmp_word as *const Code);
//                 assert_eq!(*res.get() == TRUE, $ok);
//             }
//         }};
//     }

//     cmp!(int_eq,      5, 5, true);
//     cmp!(int_neq,     5, 6, true);
//     cmp!(int_smaller, 2, 3, true);
//     cmp!(int_bigger,  9, 4, true);
//     cmp!(int_le,      4, 4, true);
//     cmp!(int_ge,      7, 2, true);

//     for _ in 0..3 { vm.param_stack.pop().unwrap(); }

//     /* ---- logical AND / OR / XOR ---- */
//     let mut lhs = UnsafeCell::new(PalData { bool: FALSE });
//     let mut rhs = UnsafeCell::new(PalData { bool: FALSE });

//     let mut bool_code = [
//         Code::basic(pick, 1),
//         Code::basic(pick, 1),
//         Code::basic(bool_and, 0), // placeholder
//         Code::basic(param_drop, 1),
//         Code::basic(ret, 0),
//     ];
//     let bool_word = Code::word(&bool_code);

//     vm.param_stack.push(lhs.get() as *mut _).unwrap();
//     vm.param_stack.push(rhs.get() as *mut _).unwrap();

//     macro_rules! logic {
//         ($builtin:path, $l:expr, $r:expr, $expect:expr) => {{
//             *lhs.get_mut() = PalData { bool: $l };
//             *rhs.get_mut() = PalData { bool: $r };
//             bool_code[2]   = Code::basic($builtin, 0);
//             unsafe { 
//                 vm.excute_code(&bool_word as *const Code);
//                 assert_eq!((*lhs.get()).bool == TRUE, $expect); 
//             };
            
//         }};
//     }

//     logic!(bool_and, TRUE,  TRUE,  true);
//     logic!(bool_or,  FALSE, TRUE,  true);
//     logic!(bool_xor, TRUE,  TRUE,  false);

//     vm.param_stack.pop().unwrap();
//     vm.param_stack.pop().unwrap();

//     /* ---- unary NOT ---- */
//     let mut v = UnsafeCell::new(TRUE);
//     let not_word = Code::word(&[
//         Code::basic(bool_not, 0),
//         Code::basic(param_drop, 1),
//         Code::basic(ret, 0),
//     ]);

//     vm.param_stack.push(v.get() as *mut _).unwrap();
//     unsafe { 
//         vm.excute_code(&not_word as *const Code);
//         assert_eq!(*v.get_mut() == FALSE, true);
//     }
// }