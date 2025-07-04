#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)] //we have a global safey doc
#![allow(clippy::assign_op_pattern)] //we use macros dont really dont need this
#![allow(clippy::unnecessary_cast)]

use crate::vm::Code;
use crate::vm::Vm;
use crate::{PalBool, PalData};
use core::ptr::{copy, copy_nonoverlapping};
use core::sync::atomic::Ordering;

/*════════════════ helpers ════════════════*/

#[inline(always)]
unsafe fn param(code_ptr: *const Code) -> *const Code {
    unsafe { (*code_ptr).param.load(Ordering::Relaxed) as *const _ }
}

#[inline(always)]
pub unsafe fn unwrap_under<T>(v: Option<T>) -> T {
    #[cfg(feature = "unchecked_underflow")]
    unsafe {
        v.unwrap_unchecked()
    }
    #[cfg(not(feature = "unchecked_underflow"))]
    {
        v.expect("stack underflow")
    }
}

#[inline(always)]
pub unsafe fn unwrap_over<T>(v: Option<T>) -> T {
    #[cfg(feature = "unchecked_overflow")]
    unsafe {
        v.unwrap_unchecked()
    }
    #[cfg(not(feature = "unchecked_overflow"))]
    {
        v.expect("stack overflow")
    }
}

macro_rules! pop {
    ($vm:expr) => {
        unwrap_under($vm.param_stack.pop())
    };
}
macro_rules! push {
    ($vm:expr, $v:expr) => {
        unwrap_over($vm.param_stack.push($v).ok())
    };
}
macro_rules! spot {
    ($vm:expr, $i:expr) => {
        unwrap_under($vm.param_stack.spot_raw($i))
    };
}
macro_rules! dspot {
    ($vm:expr, $i:expr) => {
        unwrap_under($vm.data_stack.spot_raw($i))
    };
}

/*════════════════ macro to define a builtin ════════════════*/

// #[macro_export]
// macro_rules! vm_trace {
//     ($code:expr) => {
//         #[cfg(feature = "trace_vm")]
//         {
//             // use whatever logging facility you like
//             println!("executing {}", $name);
//         }
//     };
// }

/*═════════════════════════════ main ════════════════════════════════*/

pub unsafe extern "C-unwind" fn no_op(code_ptr: *const Code, _: &mut Vm) -> *const Code {
    code_ptr
}

pub unsafe extern "C-unwind" fn log_bytes(code_ptr: *const Code, vm: &mut Vm) -> *const Code { unsafe {
    let s = param(code_ptr) as *const *const [u8];
    vm.output.write_all(&**s).unwrap();
    code_ptr
}}


/* ───────────────── memory / frame ops ───────────────── */

pub unsafe extern "C-unwind" fn inject(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let n = param(code_ptr) as usize;
        let src = pop!(vm) as *const u8;
        let dst = (*spot!(vm, 0)) as *mut u8;

        #[cfg(feature = "trace_vm")]
        println!("injecting {n} bytes from {src:?} to {dst:?}");

        copy_nonoverlapping(src, dst, n);
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn inject_non_unique(
    code_ptr: *const Code,
    vm: &mut Vm,
) -> *const Code {
    unsafe {
        let n = param(code_ptr) as usize;
        let src = pop!(vm) as *const u8;
        let dst = (*spot!(vm, 0)) as *mut u8;

        #[cfg(feature = "trace_vm")]
        println!("copying {n} bytes from {src:?} to {dst:?}");

        copy(src, dst, n);
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn frame_alloc(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        unwrap_over(vm.data_stack.alloc(param(code_ptr) as usize));
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn frame_free(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        unwrap_under(vm.data_stack.free(param(code_ptr) as usize));
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn param_drop(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        unwrap_under(vm.param_stack.free(param(code_ptr) as usize));
        code_ptr
    }
}

/* ───────────────── stack access ───────────────── */

pub unsafe extern "C-unwind" fn push_local(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let idx = param(code_ptr) as usize;
        let p = dspot!(vm, idx);

        #[cfg(feature = "trace_vm")]
        println!("pushing local {p:?}");

        push!(vm, p);
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn push_var(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        push!(vm, param(code_ptr) as *mut PalData);
        code_ptr
    }
}

pub unsafe extern "C-unwind" fn pick(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let p = *spot!(vm, param(code_ptr) as usize);
        push!(vm, p);
        code_ptr
    }
}

/* ───────────────── control flow ───────────────── */

pub unsafe extern "C-unwind" fn branch(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let cond = pop!(vm) as *const PalBool;
        let offset = param(code_ptr) as isize;

        #[cfg(feature = "trace_vm")]
        {
            let ans = *cond;
            println!("branching based on {cond:?} got {ans}");
        }

        if *cond {
            code_ptr.wrapping_offset(offset)
        } else {
            code_ptr
        }
    }
}

pub unsafe extern "C-unwind" fn _if(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let cond = pop!(vm) as *const PalBool;
        let target = param(code_ptr) as *const Code;

        #[cfg(feature = "trace_vm")]
        {
            let ans = *cond;
            println!("branching based on {cond:?} got {ans}");
        }

        if *cond { target } else { code_ptr }
    }
}

//this is for platforms without a JIT
//if there is a JIT allways prefer JITing a simple return_x and writing it to the f side
pub unsafe extern "C-unwind" fn maybe_backpatch(
    code_ptr: *const Code,
    _vm: &mut Vm,
) -> *const Code {
    unsafe {
        let p = param(code_ptr);
        if p.is_null() { code_ptr } else { p }
    }
}

pub unsafe extern "C-unwind" fn jump(code_ptr: *const Code, _vm: &mut Vm) -> *const Code {
    unsafe { code_ptr.wrapping_offset(param(code_ptr) as isize) }
}

pub unsafe extern "C-unwind" fn tail_call(code_ptr: *const Code, _vm: &mut Vm) -> *const Code {
    unsafe { param(code_ptr) }
}

///this call only works on words which are not a buildin
///this is because strictly speaking a buildin is not a valid function (other than ret and tail_call)
///buildins by themselves do not inform the excutor on when to stop
pub unsafe extern "C-unwind" fn call_dyn(call_site: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let target = (*pop!(vm)).code;

        #[cfg(feature = "trace_vm")]
        println!("calling {target:?} dynamically from {call_site:?}");

        unwrap_over(vm.return_stack.push(call_site).ok());
        param(target).wrapping_sub(1)
    }
}

///we assume the code is good for threaded excution. also see [`call_dyn`]
pub unsafe extern "C-unwind" fn call_dyn_threaded(_code: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let code = (*pop!(vm)).code;

        #[cfg(feature = "trace_vm")]
        println!("calling {code:?} dynamically");

        vm.execute_threaded(code)
    }
}

// no unsafe needed, just null return
pub extern "C-unwind" fn ret(_: *const Code, _: &mut Vm) -> *const Code {
    core::ptr::null()
}

/* ───────────────── output ───────────────── */

#[cfg(feature = "std")]
pub unsafe extern "C-unwind" fn const_print(code_ptr: *const Code, _vm: &mut Vm) -> *const Code {
    unsafe {
        use std::ffi::CStr;

        let cstr = CStr::from_ptr(param(code_ptr) as *const i8);
        print!("{}", cstr.to_str().unwrap());
        code_ptr
    }
}

/* ───────────────── arithmetic helpers ───────────────── */

#[inline(always)]
unsafe extern "C-unwind" fn bin_int_op(vm: &mut Vm, op: impl Fn(i64, i64) -> i64) {
    unsafe {
        let rhs = pop!(vm) as *const PalData;
        let lhs = (*spot!(vm, 0)) as *mut PalData;
        let v = op((*lhs).int, (*rhs).int);
        (*lhs).int = v;
    }
}

macro_rules! arith_fn {
    ($fname:ident, $op:tt) => {
        pub unsafe extern "C-unwind" fn $fname(code_ptr:*const Code, vm:&mut Vm) -> *const Code { unsafe {
            bin_int_op(vm, |a,b| a $op b);
            code_ptr
        }}
    };
}
arith_fn!(int_add, +);
arith_fn!(int_sub, -);
arith_fn!(int_mul, *);
arith_fn!(int_div, /);
arith_fn!(int_mod, %);
arith_fn!(int_shl, <<);
arith_fn!(int_shr, >>);
arith_fn!(int_and, &);
arith_fn!(int_or , |);
arith_fn!(int_xor, ^);

/* ───────────────── comparisons ───────────────── */

#[inline(always)]
unsafe extern "C-unwind" fn cmp_op(vm: &mut Vm, op: impl Fn(i64, i64) -> bool) {
    unsafe {
        let rhs = pop!(vm) as *const PalData;
        let lhs = pop!(vm) as *const PalData;
        let dst = (*spot!(vm, 0)) as *mut PalBool;
        *dst = op((*lhs).int, (*rhs).int);
    }
}

macro_rules! cmp_fn {
    ($fname:ident, $op:tt) => {
        pub unsafe extern "C-unwind" fn $fname(code_ptr:*const Code, vm:&mut Vm) -> *const Code { unsafe {
            cmp_op(vm, |a,b| a $op b);
            code_ptr
        }}
    };
}
cmp_fn!(int_eq,  ==);
cmp_fn!(int_neq, !=);
cmp_fn!(int_smaller, <);
cmp_fn!(int_bigger, >);
cmp_fn!(int_le, <=);
cmp_fn!(int_ge, >=);

/* ───────────────── boolean logic ───────────────── */

macro_rules! bool_logic {
    ($fname:ident, $op:tt) => {
        pub unsafe extern "C-unwind" fn $fname(code_ptr:*const Code, vm:&mut Vm) -> *const Code { unsafe {
            let src = pop!(vm)  as *const PalBool;
            let dst = (*spot!(vm,0)) as *mut   PalBool;
            *dst = (*src) $op (*dst);
            code_ptr
        }}
    };
}
bool_logic!(bool_and, &);
bool_logic!(bool_or,  |);
bool_logic!(bool_xor, ^);

pub unsafe extern "C-unwind" fn bool_not(code_ptr: *const Code, vm: &mut Vm) -> *const Code {
    unsafe {
        let dst = (*spot!(vm, 0)) as *mut PalBool;
        *dst = !*dst;
        code_ptr
    }
}
