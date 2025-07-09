use crate::types::Type;
use crate::types::TypeInner;
use crate::ir::Exe;
use crate::types::SigItem;
use crate::types::WRITE_FLAG;
use crate::types::READ_FLAG;
use crate::ir::RuntimeCode;
use crate::vm::BuildinFunc;
use crate::Code;
use crate::lex::Lex;
use crate::types::BasicType;


/// Ensure the canonical 64‑bit signed integer type exists and return it.
pub fn get_int_type<'lex>(lex: &mut Lex<'lex>) -> &'lex Type<'lex> {
    // We *key* the type map by `TypeInner`, so use the Basic variant as
    // the lookup key.  If missing, allocate a fresh entry.
    if let Some(t) = lex.type_map.get(&TypeInner::Basic(BasicType::Int as i32)) {
        return t;
    }

    // Create a new "int" in the types arena.
    let ty = Type {
        inner: TypeInner::Basic(BasicType::Int as i32),
        size: size_of::<i64>() as i32,
        cells: 1,
        name: "int",
    };
    let slot = lex
        .types_mem
        .save(ty)
        .expect("types arena full while inserting int");
    // Insert into the hash‑map for future look‑ups.
    lex.type_map.insert(&slot.inner, slot);
    slot
}

/// Allocate the two-cell threaded word `[primitive, ret]` inside the lexer’s
/// code arena and return a reference to the slice.
pub fn add_buildin<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
) -> &'lex [Code] {
    // SAFETY: code arena comes from a bump allocator with 'lex lifetime
    let start = lex.code_mem.check_point();
    lex.code_mem.save(Code::basic(primitive, 0)).expect("code arena full");
    lex.code_mem.save(Code::basic(crate::buildins::ret, 0)).expect("code arena full");
    lex.code_mem.index_checkpoint(start)
}

/// Convenience generator for the integer binary operators
fn make_bin_int_op<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
) -> RuntimeCode<'lex> {
    let tp = get_int_type(lex);
    let inputs  = lex
        .comp_data_mem
        .save([SigItem { tp, permissions: READ_FLAG }])
        .expect("out of comp mem")
        .leak();
    let outputs = lex
        .comp_data_mem
        .save([SigItem { tp, permissions: READ_FLAG | WRITE_FLAG }])
        .expect("out of comp mem")
        .leak();


    RuntimeCode {
        exe: Exe::Inlined(add_buildin(lex, primitive)),
        input_sig: inputs,
        output_sig: outputs,
    }
}

// Public constructors the tests will use
pub fn make_int_add<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
    make_bin_int_op(lex, crate::buildins::int_add)
}
pub fn make_int_sub<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
    make_bin_int_op(lex, crate::buildins::int_sub)
}


// Positive: SigStack accepts and the VM computes the right value
#[test]
fn int_add_sig_and_runtime_ok() {
use crate::vm::CompMode;
use core::mem::ManuallyDrop;
use crate::ir::CompContext;
use crate::vm::VmEasyMemory;
use crate::lex::LexEasyMemory;

    // --- plumbing ------------------------------------------------------
    let mut vm_mem  = VmEasyMemory::<128>::new();
    let mut lex_mem = LexEasyMemory::new();

    let lex: &mut Lex = &mut ManuallyDrop::new(lex_mem.make_lex());
    let lex_p = lex as *mut _;
    let lex = unsafe { &mut *lex_p };

    // Build wrapper and a CompContext
    let rc = make_int_add(lex);
    let ctx = CompContext::new(lex, None);

    // Prepare a VM with two ints on the param stack
    let mut vm = vm_mem.make_vm();

    let mut a = 3_i64;
    let mut b = 4_i64;

    let a_ptr = &mut a as *mut _ as *mut _;
    let b_ptr = &mut b as *mut _ as *mut _;

    vm.param_stack.push(a_ptr).unwrap();
    vm.param_stack.push(b_ptr).unwrap();
    vm.comp=CompMode::Run(ctx);

    let ctx = vm.comp.get_comp_crash();
    let lex = &mut ctx.lex;
    let sig_stack = &mut ctx.immidate_stack;

    // --- type-checking --------------------------------------------------
    // Build a SigStack that reflects the two i64s we just pushed
    let v1 = sig_stack.add_owned_var(get_int_type(lex));
    let v2 = sig_stack.add_owned_var(get_int_type(lex));
    sig_stack.push(v1);
    sig_stack.push(v2);


    // --- execute via comp_run_checked ----------------------------------
    unsafe { rc.comp_run_checked(&mut vm).unwrap() };

    // Result should be 7 on the param stack
    assert_eq!(a, 7);

    //and we should also have a ontop
    assert_eq!(*vm.param_stack.peek().unwrap(),a_ptr);

    // --- cleanup -------------------------------------------------------
    core::mem::drop(vm);
    unsafe {
        core::ptr::drop_in_place(lex_p);
    }
}

// Negative: forget one argument ⇒ SigStack must reject
#[test]
fn int_add_sig_rejects_missing_rhs() {
use crate::types::SigStack;
use crate::lex::LexEasyMemory;

    let mut lex_mem = LexEasyMemory::new();
    let mut lex = lex_mem.make_lex();

    let mut sig_stack = SigStack::default();
    let v = sig_stack.add_owned_var(get_int_type(&mut lex));
    sig_stack.push(v);          // Only one operand

    // Dummy lex just to build a wrapper
    let mut lex_mem = LexEasyMemory::new();
    let mut lex     = lex_mem.make_lex();
    let rc          = make_int_add(&mut lex);

    sig_stack.call_sig(&rc.input_sig,&rc.output_sig).unwrap_err();
}


// use core::mem::size_of;


// /*──────────────────── type helpers ────────────────────*/

// /// Intern the canonical 64‑bit *int* type and return a reference.
// pub fn get_int_type<'lex>(lex: &mut Lex<'lex>) -> &'lex Type<'lex> {
//     get_basic_type(lex, BasicType::Int, size_of::<i64>(), "int")
// }

// /// Intern the canonical *bool* type and return a reference.
// pub fn get_bool_type<'lex>(lex: &mut Lex<'lex>) -> &'lex Type<'lex> {
//     get_basic_type(lex, BasicType::Bool, size_of::<bool>(), "bool")
// }

// fn get_basic_type<'lex>(
//     lex: &mut Lex<'lex>,
//     tag: BasicType,
//     size: usize,
//     name: &'static str,
// ) -> &'lex Type<'lex> {
//     let key = TypeInner::Basic(tag as i32);
//     if let Some(t) = lex.type_map.get(&key) {
//         return t;
//     }
//     let ty = Type {
//         inner: key,
//         size: size as i32,
//         cells: 1,
//         name,
//     };
//     let slot = lex
//         .types_mem
//         .save(ty)
//         .expect("types arena full while interning basic type");
//     lex.type_map.insert(&slot.inner, slot);
//     slot
// }



// /*──────────────────── signature helpers ────────────────────*/

// /// Allocate `SigItem` slices in `comp_data_mem` and build the `RuntimeCode`.
// fn build_rc<'lex>(
//     lex: &mut Lex<'lex>,
//     prim: BuildinFunc,
//     inputs: &[SigItem<'lex>],
//     outputs: &[SigItem<'lex>],
// ) -> RuntimeCode<'lex> 
// where SigItem<'lex>:Copy
// {
//     let in_slice = lex
//         .comp_data_mem
//         .save_slice(inputs)
//         .expect("comp_data arena full (inputs)")
//         .leak();
//     let out_slice = lex
//         .comp_data_mem
//         .save_slice(outputs)
//         .expect("comp_data arena full (outputs)")
//         .leak();

//     RuntimeCode {
//         exe: Exe::Inlined(add_buildin(lex, prim)),
//         input_sig: in_slice,
//         output_sig: out_slice,
//     }
// }

// /*──────────────────── macro generators ────────────────────*/

// macro_rules! mk_rw_r {
//     ($( $fn_name:ident => $prim:path ),* $(,)?) => {
//         $(
//         #[allow(non_snake_case)]
//         pub fn $fn_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
//             let int = get_int_type(lex);
//             let inputs  = [SigItem { tp: int, permissions: READ_FLAG }];
//             let outputs = [SigItem { tp: int, permissions: READ_FLAG | WRITE_FLAG }];
//             build_rc(lex, $prim, &inputs, &outputs)
//         }
//         )*
//     };
// }

// macro_rules! mk_w_rr {
//     ($( $fn_name:ident => $prim:path ),* $(,)?) => {
//         $(
//         pub fn $fn_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
//             let int  = get_int_type(lex);
//             let bool_t = get_bool_type(lex);
//             let inputs  = [
//                 // rhs, lhs (pop order) – both read‑only
//                 SigItem { tp: int, permissions: READ_FLAG },
//                 SigItem { tp: int, permissions: READ_FLAG },
//             ];
//             let outputs = [SigItem { tp: bool_t, permissions: WRITE_FLAG | READ_FLAG }];
//             build_rc(lex, $prim, &inputs, &outputs)
//         }
//         )*
//     };
// }

// macro_rules! mk_rw_r_bool {
//     ($( $fn_name:ident => $prim:path ),* $(,)?) => {
//         $(
//         pub fn $fn_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
//             let bool_t = get_bool_type(lex);
//             let inputs  = [SigItem { tp: bool_t, permissions: READ_FLAG }];
//             let outputs = [SigItem { tp: bool_t, permissions: READ_FLAG | WRITE_FLAG }];
//             build_rc(lex, $prim, &inputs, &outputs)
//         }
//         )*
//     };
// }

// // Unary bool op (NOT): dst RW → dst RW
// pub fn make_bool_not<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
//     let bool_t = get_bool_type(lex);
//     let inputs: [SigItem; 0] = [];
//     let outputs = [SigItem { tp: bool_t, permissions: READ_FLAG | WRITE_FLAG }];
//     build_rc(lex, crate::buildins::bool_not, &inputs, &outputs)
// }

// /*──────────────────── generate the wrappers ────────────────────*/

// mk_rw_r! {
//     make_int_add  => crate::buildins::int_add,
//     make_int_sub  => crate::buildins::int_sub,
//     make_int_mul  => crate::buildins::int_mul,
//     make_int_div  => crate::buildins::int_div,
//     make_int_mod  => crate::buildins::int_mod,
//     make_int_shl  => crate::buildins::int_shl,
//     make_int_shr  => crate::buildins::int_shr,
//     make_int_and  => crate::buildins::int_and,
//     make_int_or   => crate::buildins::int_or,
//     make_int_xor  => crate::buildins::int_xor,
// }

// mk_w_rr! {
//     make_int_eq      => crate::buildins::int_eq,
//     make_int_neq     => crate::buildins::int_neq,
//     make_int_smaller => crate::buildins::int_smaller,
//     make_int_bigger  => crate::buildins::int_bigger,
//     make_int_le      => crate::buildins::int_le,
//     make_int_ge      => crate::buildins::int_ge,
// }

// mk_rw_r_bool! {
//     make_bool_and => crate::buildins::bool_and,
//     make_bool_or  => crate::buildins::bool_or,
//     make_bool_xor => crate::buildins::bool_xor,
// }

// /*──────────────────── tests ────────────────────*/

// #[cfg(test)]
// mod tests {
//     use crate::types::SigStack;
// use super::*;
//     use crate::ir::{CompContext};
//     use crate::lex::LexEasyMemory;
//     use crate::vm::{CompMode, VmEasyMemory};
//     use core::mem::ManuallyDrop;


//     /* positive test for int_add ------------------------------------------------*/
//     #[test]
//     fn int_add_ok() {
//         let mut vm_mem = VmEasyMemory::<1024>::new();
//         let mut lex_mem = LexEasyMemory::new();

//         let lex: &mut Lex = &mut ManuallyDrop::new(lex_mem.make_lex());
//         let lex_p = lex as *mut _;
//         let lex = unsafe { &mut *lex_p };


//         let mut vm = vm_mem.make_vm();
//         vm.comp = CompMode::Run(CompContext::new(lex,None));
//         let lex = &mut vm.comp.get_comp_crash().lex;


//         let rc = make_int_add(lex);

//         // Put lhs / rhs on stack (pointers to data)
//         let mut a = 2_i64;
//         let mut b = 5_i64;
//         let a_ptr = &mut a as *mut _ as *mut _;
//         let b_ptr = &mut b as *mut _ as *mut _;

//         vm.param_stack.push(a_ptr).unwrap();
//         vm.param_stack.push(b_ptr).unwrap();

//         // compile‑time stack simulation
//         let mut ss = SigStack::default();
//         let v1 = ss.add_owned_var(get_int_type(lex));
//         let v2 = ss.add_owned_var(get_int_type(lex));
//         ss.push(v1);
//         ss.push(v2);
//         ss.call_sig(rc.input_sig, rc.output_sig).unwrap();

//         unsafe { rc.comp_run_checked(&mut vm).unwrap() };
//         assert_eq!(a, 7);

//         //cleanup
//         core::mem::drop(vm);
//         unsafe {
//             core::ptr::drop_in_place(lex_p);
//         }
//     }

//     /* negative test: missing rhs ------------------------------------------------*/
//     #[test]
//     fn int_add_reject_missing_rhs() {
//         let mut lex_mem = LexEasyMemory::new();
//         let mut lex = lex_mem.make_lex();
//         let rc  = make_int_add(&mut lex);

//         let mut ss = SigStack::default();
//         let v = ss.add_owned_var(get_int_type(&mut lex));
//         ss.push(v);
//         assert!(ss.call_sig(rc.input_sig, rc.output_sig).is_err());
//     }
// }
