use crate::ir::Word;
use crate::types::RwT;
use crate::types::UNIQUE_FLAG;
use crate::PalBool;
use crate::PalInt;
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

pub fn register_buildins<'lex>(lex: &mut Lex<'lex>) {
    macro_rules! reg {
        ($name:expr, $maker:expr) => {
            {
                let rt = $maker(lex);
                let word = Word {
                    name: $name,
                    runtime: rt,
                    immidate: None,
                };
                lex.words.insert(word.name, word);
            }
        };
    }

    // Integer binary ops
    reg!("int_add", make_int_add);
    reg!("int_sub", make_int_sub);
    reg!("int_mul", make_int_mul);
    reg!("int_div", make_int_div);
    reg!("int_mod", make_int_mod);
    reg!("int_shl", make_int_shl);
    reg!("int_shr", make_int_shr);
    reg!("int_and", make_int_and);
    reg!("int_or",  make_int_or);
    reg!("int_xor", make_int_xor);

    // Integer comparisons
    reg!("int_eq",      make_int_eq);
    reg!("int_neq",     make_int_neq);
    reg!("int_smaller", make_int_smaller);
    reg!("int_bigger",  make_int_bigger);
    reg!("int_le",      make_int_le);
    reg!("int_ge",      make_int_ge);

    // Boolean binary ops
    reg!("bool_and", make_bool_and);
    reg!("bool_or",  make_bool_or);
    reg!("bool_xor", make_bool_xor);

    reg!("bool_not", make_bool_not);


    // Bool injects
    reg!("bool_inject", make_bool_inject);
    reg!("bool_inject_non_unique", make_bool_inject_non_unique);

    // Int injects
    reg!("int_inject", make_int_inject);
    reg!("int_inject_non_unique", make_int_inject_non_unique);
}


pub fn get_int_type<'lex>(lex: &mut Lex<'lex>) -> &'lex Type<'lex> {
    intern_basic_type(lex, BasicType::Int as i32, "int", size_of::<PalInt>())
}

/// Lazily intern the canonical boolean type (`PalBool`) and return it.
pub fn get_bool_type<'lex>(lex: &mut Lex<'lex>) -> &'lex Type<'lex> {
    intern_basic_type(lex, BasicType::Bool as i32, "bool", size_of::<PalBool>())
}

pub fn intern_basic_type<'lex>(
    lex: &mut Lex<'lex>,
    disc: i32,
    name: &'static str,
    size: usize,
) -> &'lex Type<'lex> {
    if let Some(t) = lex.type_map.get(&TypeInner::Basic(disc)) {
        return t;
    }
    let ty = Type {
        inner: TypeInner::Basic(disc),
        size: size as i32,
        cells: 1,
        name,
    };
    let slot = lex.types_mem.save(ty).expect("types arena full");
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

pub fn add_buildin_2<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
    param:isize
) -> &'lex [Code] {
    // SAFETY: code arena comes from a bump allocator with 'lex lifetime
    let start = lex.code_mem.check_point();
    lex.code_mem.save(Code::basic(primitive, param)).expect("code arena full");
    lex.code_mem.save(Code::basic(crate::buildins::ret, param)).expect("code arena full");
    lex.code_mem.index_checkpoint(start)
}


//------bins---------------------------------

/// Convenience generator for the integer binary operators
pub fn make_bin_int_op<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
) -> RuntimeCode<'lex>
where SigItem<'lex> : Copy,
 {
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

macro_rules! bin_int_word {
    ($func_name:ident, $builtin:ident) => {
        pub fn $func_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
            make_bin_int_op(lex, crate::buildins::$builtin)
        }
    };
}

bin_int_word!(make_int_add, int_add);
bin_int_word!(make_int_sub, int_sub);
bin_int_word!(make_int_mul, int_mul);
bin_int_word!(make_int_div, int_div);
bin_int_word!(make_int_mod, int_mod);
bin_int_word!(make_int_shl, int_shl);
bin_int_word!(make_int_shr, int_shr);
bin_int_word!(make_int_and, int_and);
bin_int_word!(make_int_or , int_or);
bin_int_word!(make_int_xor, int_xor);


//-----comps---------------------------

pub fn make_cmp_int_op<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
) -> RuntimeCode<'lex>
where SigItem<'lex> : Copy,
 {
    let tp = get_int_type(lex);
    let inputs  = lex
        .comp_data_mem
        .save([
            SigItem { tp, permissions: READ_FLAG },
            SigItem { tp, permissions: READ_FLAG },
        ])
        .expect("out of comp mem")
        .leak();

    let tp = get_bool_type(lex);
    let outputs = lex
        .comp_data_mem
        .save([SigItem { tp, permissions: WRITE_FLAG }])
        .expect("out of comp mem")
        .leak();


    RuntimeCode {
        exe: Exe::Inlined(add_buildin(lex, primitive)),
        input_sig: inputs,
        output_sig: outputs,
    }
}

macro_rules! cmp_int_word {
    ($func_name:ident, $builtin:ident) => {
        pub fn $func_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
            make_cmp_int_op(lex, crate::buildins::$builtin)
        }
    };
}

cmp_int_word!(make_int_eq     , int_eq);
cmp_int_word!(make_int_neq    , int_neq);
cmp_int_word!(make_int_smaller, int_smaller);
cmp_int_word!(make_int_bigger , int_bigger);
cmp_int_word!(make_int_le     , int_le);
cmp_int_word!(make_int_ge     , int_ge);

//------bool logic---------------------------
pub fn make_bin_bool_op<'lex>(
    lex: &mut Lex<'lex>,
    primitive: BuildinFunc,
) -> RuntimeCode<'lex>
where SigItem<'lex> : Copy,
 {
    let tp = get_bool_type(lex);
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

macro_rules! bin_bool_word {
    ($func_name:ident, $builtin:ident) => {
        pub fn $func_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
            make_bin_bool_op(lex, crate::buildins::$builtin)
        }
    };
}

bin_bool_word!(make_bool_and, bool_and);
bin_bool_word!(make_bool_or , bool_or );
bin_bool_word!(make_bool_xor, bool_xor);

pub fn make_bool_not<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> where SigItem<'lex> : Copy{
    let tp = get_bool_type(lex);
    let outputs = lex
        .comp_data_mem
        .save([SigItem { tp, permissions: READ_FLAG | WRITE_FLAG }])
        .expect("out of comp mem")
        .leak();

    RuntimeCode { exe: Exe::Inlined(add_buildin(lex, crate::buildins::bool_not)), input_sig: &[], output_sig: outputs }
}

//-------------injects----------------------------
pub fn make_assign_op<'lex>(lex: &mut Lex<'lex>,tp:&'lex Type<'lex>,permissions:RwT, prim: BuildinFunc) -> RuntimeCode<'lex> where SigItem<'lex> : Copy{
    let outputs = lex
        .comp_data_mem
        .save([SigItem { tp, permissions }])
        .expect("out of comp mem")
        .leak();

    let inputs = lex
        .comp_data_mem
        .save([SigItem { tp, permissions: READ_FLAG }])
        .expect("out of comp mem")
        .leak();

    RuntimeCode { exe: Exe::Inlined(add_buildin_2(lex, prim,tp.size as isize)), input_sig: inputs, output_sig: outputs }
}

macro_rules! bin_assign_word {
    ($func_name:ident,$make_tp:expr,$perms:expr, $builtin:ident) => {
        pub fn $func_name<'lex>(lex: &mut Lex<'lex>) -> RuntimeCode<'lex> {
            let tp = $make_tp(lex);
            make_assign_op(lex,tp,$perms, crate::buildins::$builtin)
        }
    };
}

bin_assign_word!(make_bool_inject,get_bool_type,UNIQUE_FLAG | WRITE_FLAG,inject);
bin_assign_word!(make_bool_inject_non_unique,get_bool_type, WRITE_FLAG,inject_non_unique);

bin_assign_word!(make_int_inject,get_int_type,UNIQUE_FLAG | WRITE_FLAG,inject);
bin_assign_word!(make_int_inject_non_unique,get_int_type, WRITE_FLAG,inject_non_unique);



/*──────────────────────── tests for the new wrapper families ─────────────────────*/

#[cfg(test)]
mod sig_tests {
    use crate::types::SigStack;
use super::*;
    use crate::{
        ir::CompContext,
        lex::LexEasyMemory,
        vm::{CompMode, VmEasyMemory},
    };
    use core::mem::ManuallyDrop;

    // Positive: SigStack accepts and the VM computes the right value
    #[test]
    fn int_add_sig_and_runtime_ok() {
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

        let mut a = 3 as PalInt;
        let mut b = 4 as PalInt;

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

    /*──────────── integer comparison family ────────────*/

    #[test]
    fn int_eq_sig_and_runtime_ok() {
        let mut vm_mem  = VmEasyMemory::<128>::new();
        let mut lex_mem = LexEasyMemory::new();

        /* build lex + wrapper */
        let lex: &mut Lex = &mut ManuallyDrop::new(lex_mem.make_lex());
        let lex_p = lex as *mut _;
        let lex   = unsafe { &mut *lex_p };

        let rc  = make_int_eq(lex);
        let ctx = CompContext::new(lex, None);

        /* VM setup */
        let mut vm = vm_mem.make_vm();
        let mut dst = false as PalBool;
        let mut lhs = 5     as PalInt;
        let mut rhs = 5     as PalInt;

        vm.param_stack.push(&mut dst as *mut _ as *mut _).unwrap(); // dest (bottom)
        vm.param_stack.push(&mut lhs as *mut _ as *mut _).unwrap(); // lhs
        vm.param_stack.push(&mut rhs as *mut _ as *mut _).unwrap(); // rhs  (top)
        vm.comp = CompMode::Run(ctx);

        /* SigStack mirrors the runtime stack */
        let ctx       = vm.comp.get_comp_crash();
        let lex       = &mut ctx.lex;
        let sig_stack = &mut ctx.immidate_stack;

        let v_dst = sig_stack.add_owned_var(get_bool_type(lex));
        sig_stack.push(v_dst);
        let v_lhs = sig_stack.add_owned_var(get_int_type(lex));
        sig_stack.push(v_lhs);
        let v_rhs = sig_stack.add_owned_var(get_int_type(lex));
        sig_stack.push(v_rhs);

        unsafe { rc.comp_run_checked(&mut vm).unwrap() };

        assert!(dst);                                                   // 5 == 5
        assert_eq!(*vm.param_stack.peek().unwrap(),
                   &mut dst as *mut _ as *mut _);

        /* cleanup */
        core::mem::drop(vm);
        unsafe { core::ptr::drop_in_place(lex_p); }
    }

    /*──────────── boolean binary family ────────────*/

    #[test]
    fn bool_and_sig_and_runtime_ok() {
        let mut vm_mem  = VmEasyMemory::<128>::new();
        let mut lex_mem = LexEasyMemory::new();

        let lex: &mut Lex = &mut ManuallyDrop::new(lex_mem.make_lex());
        let lex_p = lex as *mut _;
        let lex   = unsafe { &mut *lex_p };

        let rc  = make_bool_and(lex);
        let ctx = CompContext::new(lex, None);

        let mut vm  = vm_mem.make_vm();
        let mut dst = true  as PalBool;
        let mut src = false as PalBool;

        vm.param_stack.push(&mut dst as *mut _ as *mut _).unwrap(); // dest (bottom)
        vm.param_stack.push(&mut src as *mut _ as *mut _).unwrap(); // src  (top)
        vm.comp = CompMode::Run(ctx);

        let ctx       = vm.comp.get_comp_crash();
        let lex       = &mut ctx.lex;
        let sig_stack = &mut ctx.immidate_stack;

        let v_dst = sig_stack.add_owned_var(get_bool_type(lex));
        sig_stack.push(v_dst);
        let v_src = sig_stack.add_owned_var(get_bool_type(lex));
        sig_stack.push(v_src);

        unsafe { rc.comp_run_checked(&mut vm).unwrap() };

        assert!(!dst);                                                  // true & false == false
        assert_eq!(*vm.param_stack.peek().unwrap(),
                   &mut dst as *mut _ as *mut _);

        core::mem::drop(vm);
        unsafe { core::ptr::drop_in_place(lex_p); }
    }

    /*──────────── boolean unary family ────────────*/

    #[test]
    fn bool_not_sig_and_runtime_ok() {
        let mut vm_mem  = VmEasyMemory::<128>::new();
        let mut lex_mem = LexEasyMemory::new();

        let lex: &mut Lex = &mut ManuallyDrop::new(lex_mem.make_lex());
        let lex_p = lex as *mut _;
        let lex   = unsafe { &mut *lex_p };

        let rc  = make_bool_not(lex);
        let ctx = CompContext::new(lex, None);

        let mut vm  = vm_mem.make_vm();
        let mut dst = false as PalBool;

        vm.param_stack.push(&mut dst as *mut _ as *mut _).unwrap(); // dest only
        vm.comp = CompMode::Run(ctx);

        let ctx       = vm.comp.get_comp_crash();
        let lex       = &mut ctx.lex;
        let sig_stack = &mut ctx.immidate_stack;

        let v_dst = sig_stack.add_owned_var(get_bool_type(lex));
        sig_stack.push(v_dst);

        unsafe { rc.comp_run_checked(&mut vm).unwrap() };

        assert!(dst);                                                   // !false == true
        assert_eq!(*vm.param_stack.peek().unwrap(),
                   &mut dst as *mut _ as *mut _);

        core::mem::drop(vm);
        unsafe { core::ptr::drop_in_place(lex_p); }
    }
}
