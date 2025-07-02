#![allow(unused_variables)]
use std::io;
use pal_forth::vm::VmEasyMemory;
use pal_forth::lex::{LexEasyMemory};

fn main() -> io::Result<()> {
    let mut lex_mem = LexEasyMemory::new();
    let mut vm_mem = VmEasyMemory::<1024>::new();
    let lex = lex_mem.make_lex();
    let lex = vm_mem.make_vm();

    todo!();
}
