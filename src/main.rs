#![allow(unused_variables)]
use no_std_io::io;
use pal_forth::lex::LexEasyMemory;
use pal_forth::vm::VmEasyMemory;

fn main() -> io::Result<()> {
    let mut lex_mem = LexEasyMemory::new();
    let mut vm_mem = VmEasyMemory::<1024>::new();
    let lex = lex_mem.make_lex();
    let lex = vm_mem.make_vm();

    todo!();
}
