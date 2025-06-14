use std::ptr;
use crate::PalData;
use crate::stack::StackRef;

pub type BuildinFunc =  for<'vm,'a> unsafe extern "C-unwind" fn(*const Code,&mut Vm<'vm>) -> *const Code;

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Buildin{
	pub f: BuildinFunc,
	pub param:*const Code,
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Code{
	Buildin(Buildin),
	Derived(*const Code)
}

impl Code{
	pub fn basic(f:BuildinFunc,v:isize)->Self{
		Code::Buildin(Buildin{f,param: v as *const Code})
	}
	pub fn word(c:&[Code])->Self{
		Code::Derived(c as *const [_] as *const _)
	}

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
	pub param_stack:StackRef<'a, *mut PalData> ,
	pub data_stack:StackRef<'a, PalData>,
}

impl<'vm> Vm<'vm> {
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
