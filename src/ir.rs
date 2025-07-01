use crate::types::SigError;
use crate::types::SigStack;
use crate::types::SigItem;
use crate::vm::Vm;
use core::cell::UnsafeCell;
use crate::Code;


pub struct Word<'lex>{
    pub name:&'lex str,
    pub runtime:RuntimeCode<'lex>,
    pub immidate:Option<&'lex UnsafeCell<Code>>,
}

pub struct RuntimeCode<'lex>{
	exe:&'lex UnsafeCell<Code>,
	pub input_sig:&'lex [SigItem<'lex>],
	pub output_sig:&'lex [SigItem<'lex>],
}

impl From<RuntimeCode<'_>> for *const Code{
fn from(x: RuntimeCode<'_>) -> Self { x.exe.get() }
}

impl<'lex> RuntimeCode<'lex> {
	pub unsafe fn run(&self,vm:&mut Vm){
		unsafe{
			vm.execute_code(self.exe.get())
		}
	}

	#[allow(dead_code)]
	fn check_sig(&self,sig:&mut SigStack<'_,'lex>)->Result<(),SigError<'lex>>{
		sig.call_sig(self.output_sig,self.input_sig)
	}
}