use crate::types::SigItem;
use crate::vm::Vm;
use core::cell::UnsafeCell;
use crate::Code;


pub struct IRInst<'lex>{
	pub name: &'lex str,
	exe:&'lex UnsafeCell<Code>,
	sig:&'lex [SigItem<'lex>],
}

impl From<IRInst<'_>> for *const Code{
fn from(x: IRInst<'_>) -> Self { x.exe.get() }
}

impl IRInst<'_> {
	pub unsafe fn run(&self,vm:&mut Vm){
		unsafe{
			vm.execute_code(self.exe.get())
		}
	}
}