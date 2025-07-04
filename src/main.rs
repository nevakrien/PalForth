use pal_forth::input::WordStream;
use core::mem::ManuallyDrop;
use pal_forth::vm::CompMode;
use pal_forth::ir::CompEasyMemory;
use pal_forth::lex::LexEasyMemory;
use pal_forth::vm::VmEasyMemory;


fn main() -> () {
    let mut vm_mem = VmEasyMemory::<1024>::new();
    let mut lex_mem = LexEasyMemory::new();
    let mut comp_mem = CompEasyMemory::<1024>::new();


    let mut lex = ManuallyDrop::new(lex_mem.make_lex());
    let mut vm = vm_mem.make_vm();
    let mut comp = comp_mem.make_comp(&mut lex);

    let mut stream: WordStream<_, 1000> = WordStream::new(std::io::stdin());
    comp.input = Some(&mut stream);

    vm.comp=CompMode::Run(Box::new(comp));

    loop{
        match unsafe{vm.respond_to_input()}{
            Ok(_)=>{

            },
            Err(e)=>{
                println!("{e:?}", );
            }
        }
    }

}
