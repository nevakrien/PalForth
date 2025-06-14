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