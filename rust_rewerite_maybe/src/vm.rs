use std::mem::MaybeUninit;
use std::ptr;
use crate::PalData;
use crate::stack::StackRef;

pub type BuildinFunc = for<'vm> fn(*const Code,&mut Vm<'vm>) -> *const Code;

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Buildin{
	f: BuildinFunc,
	param:*const Code,
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Code{
	Buildin(&'static Buildin),
	Derived(*const Code)
}

impl Code{
	pub fn is_null(&self)->bool{
		match self{
			Code::Buildin(_) => false,
			Code::Derived(d) => d.is_null(),
		}
	}

	pub fn null()->Self{
		Code::Derived(ptr::null())
	}
}

pub struct Vm<'a> {
	param_stack:StackRef<'a, *mut PalData<'a>> ,
	data_stack:StackRef<'a, MaybeUninit<PalData<'a>>>,
}

impl Vm<'_> {
	pub fn run_code(&mut self,code:Code){
		let code_arr = [code,Code::null()];
		unsafe{self.excute_code(&code_arr as *const _);}
	}
	pub unsafe fn excute_code<'a>(&mut self,code:*const Code) -> *const Code{
		unsafe{
			match *code {
				Code::Buildin(b) => (b.f)(code,self),
				Code::Derived(mut code) => 
				loop {
					code = self.excute_code(code);
					if code.is_null(){
						return code;
					}
					code = code.add(1)
				}
			}
		}
	}
}


