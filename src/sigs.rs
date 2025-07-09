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
    let inputs  = lex.comp_data_mem.save([SigItem { tp, permissions: READ_FLAG }]);
    let outputs = lex.comp_data_mem.save([SigItem { tp, permissions: READ_FLAG | WRITE_FLAG }]);

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

    assert!(sig_stack.call_sig(&rc.input_sig,&rc.output_sig).is_err());
}
