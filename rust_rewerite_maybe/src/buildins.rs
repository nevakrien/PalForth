use crate::vm::Buildin;
use crate::vm::BuildinFunc;
use crate::vm::Vm;
use crate::vm::Code;
use std::{
    ffi::CStr,
    ptr::{copy, copy_nonoverlapping},
};
use crate::{PalData, PalBool};

/*════════════════ helpers ════════════════*/

#[inline(always)]
unsafe fn param(code_ptr: *const Code) -> *const Code { unsafe {
    match &*code_ptr {
        Code::Buildin(b) => b.param,
        _ => core::hint::unreachable_unchecked(),
    }
}}

#[inline(always)]
fn unwrap_under<T>(v: Option<T>) -> T {
    #[cfg(feature = "unchecked_underflow")]
    unsafe { v.unwrap_unchecked() }
    #[cfg(not(feature = "unchecked_underflow"))]
    { v.expect("stack underflow") }
}

#[inline(always)]
fn unwrap_over<T>(v: Option<T>) -> T {
    #[cfg(feature = "unchecked_overflow")]
    unsafe { v.unwrap_unchecked() }
    #[cfg(not(feature = "unchecked_overflow"))]
    { v.expect("stack overflow") }
}


macro_rules! pop   { ($vm:expr)        => { unwrap_under($vm.param_stack.pop()) } }
macro_rules! push  { ($vm:expr, $v:expr)=> { unwrap_over($vm.param_stack.push($v).ok())} }
macro_rules! spot  { ($vm:expr, $i:expr)=> { unwrap_under($vm.param_stack.spot_raw($i)) } }
macro_rules! dspot { ($vm:expr, $i:expr)=> { unwrap_under($vm.data_stack.spot_raw($i)) } }


/*════════════════ macro to define a builtin ════════════════*/

#[macro_export]
macro_rules! vm_trace {
    ($code:expr) => {
        #[cfg(feature = "trace_vm")]
        {
            // use whatever logging facility you like
            println!("executing {}", $name);
        }
    };
}

/* ───────────────── memory / frame ops ───────────────── */

pub unsafe extern "C-unwind" fn inject< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let n   = param(code_ptr) as usize;
    let src = pop!(vm) as *const u8;
    let dst = spot!(vm, 0) as *mut u8;
    
    #[cfg(feature = "trace_vm")]
    println!("copying {n} bytes from {src:?} to {dst:?}");
        
    copy_nonoverlapping(src, dst, n);
    code_ptr
}}

pub unsafe extern "C-unwind" fn inject_non_unique< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let n   = param(code_ptr) as usize;
    let src = pop!(vm) as *const u8;
    let dst = spot!(vm, 0) as *mut u8;
    copy(src, dst, n);
    code_ptr
}}

pub unsafe extern "C-unwind" fn frame_alloc< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    unwrap_over(vm.data_stack.alloc(param(code_ptr) as usize));
    code_ptr
}}

pub unsafe extern "C-unwind" fn frame_free< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    unwrap_under(vm.data_stack.free(param(code_ptr) as usize));
    code_ptr
}}

pub unsafe extern "C-unwind" fn param_drop< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    unwrap_under(vm.param_stack.free(param(code_ptr) as usize));
    code_ptr
}}

/* ───────────────── stack access ───────────────── */

pub unsafe extern "C-unwind" fn push_local< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let idx = param(code_ptr) as usize;
    let p   = dspot!(vm, idx);
    push!(vm, p);
    code_ptr
}}

pub unsafe extern "C-unwind" fn push_var< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    push!(vm, param(code_ptr) as *mut PalData);
    code_ptr
}}

pub unsafe extern "C-unwind" fn pick< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let p = spot!(vm, param(code_ptr) as usize);
    push!(vm, *p);
    code_ptr
}}

/* ───────────────── control flow ───────────────── */

pub unsafe extern "C-unwind" fn branch< 'vm>(code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let cond   = pop!(vm) as *const PalBool;
    let offset = param(code_ptr) as isize;
    if *cond { code_ptr.wrapping_offset(offset) } else { code_ptr }
}}

pub unsafe extern "C-unwind" fn jump< 'vm>(code_ptr: *const Code, _vm: &mut Vm<'vm>) -> *const Code { unsafe {
    code_ptr.wrapping_offset(param(code_ptr) as isize)
}}

pub unsafe extern "C-unwind" fn call_dyn< 'vm>(_code_ptr: *const Code, vm: &mut Vm<'vm>) -> *const Code {
    pop!(vm) as *const Code
}

// no unsafe needed, just null return
pub extern "C-unwind" fn ret< 'vm>(_: *const Code, _: &mut Vm<'vm>) -> *const Code {
    core::ptr::null()
}

/* ───────────────── output ───────────────── */

pub unsafe extern "C-unwind" fn const_print< 'vm>(code_ptr: *const Code, _vm: &mut Vm<'vm>) -> *const Code { unsafe {
    let cstr = CStr::from_ptr(param(code_ptr) as *const i8);
    print!("{}", cstr.to_str().unwrap());
    code_ptr
}}

/* ───────────────── arithmetic helpers ───────────────── */

#[inline(always)]
unsafe extern "C-unwind" fn bin_int_op<'vm>(
    vm: &mut Vm<'vm>,
    op: impl Fn(i64, i64) -> i64,
) { unsafe {
    let rhs = pop!(vm)   as *const PalData;
    let lhs = spot!(vm,0) as *mut  PalData;
    let v   = op((*lhs).int, (*rhs).int);
    (*lhs).int = v;
}}

macro_rules! arith_fn {
    ($fname:ident, $op:tt) => {
        pub unsafe extern "C-unwind" fn $fname<'vm>(code_ptr:*const Code, vm:&mut Vm<'vm>) -> *const Code { unsafe {
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
unsafe extern "C-unwind" fn cmp_op<'vm>(
    vm: &mut Vm<'vm>,
    op: impl Fn(i64, i64) -> bool,
) { unsafe {
    let rhs = pop!(vm) as *const PalData;
    let lhs = pop!(vm) as *const PalData;
    let dst = spot!(vm,0) as *mut PalBool;
    *dst = op((*lhs).int, (*rhs).int);
}}

macro_rules! cmp_fn {
    ($fname:ident, $op:tt) => {
        pub unsafe extern "C-unwind" fn $fname<'vm>(code_ptr:*const Code, vm:&mut Vm<'vm>) -> *const Code { unsafe {
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
        pub unsafe extern "C-unwind" fn $fname<'vm>(code_ptr:*const Code, vm:&mut Vm<'vm>) -> *const Code { unsafe {
            let src = pop!(vm)  as *const PalBool;
            let dst = spot!(vm,0) as *mut   PalBool;
            *dst = (*src) $op (*dst);
            code_ptr
        }}
    };
}
bool_logic!(bool_and, &);
bool_logic!(bool_or,  |);
bool_logic!(bool_xor, ^);

pub unsafe extern "C-unwind" fn bool_not<'vm>(code_ptr:*const Code, vm:&mut Vm<'vm>) -> *const Code { unsafe {
    let dst = spot!(vm,0) as *mut PalBool;
    *dst = !*dst;
    code_ptr
}}

/* ───────────────── helpers to build constant Buildin objects ───────────────── */

pub const fn mk_builtin(f: BuildinFunc, param: usize) -> Buildin {
    Buildin { f, param: param as *const Code }
}