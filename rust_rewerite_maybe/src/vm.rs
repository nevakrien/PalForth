use crate::stack::make_storage;
use std::mem::MaybeUninit;
use crate::PalData;
use crate::stack::StackRef;

pub type BuildinFunc =  for<'vm,'a> unsafe extern "C-unwind" fn(*const Code,&mut Vm<'vm>) -> *const Code;

#[repr(C)]
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Code{
	pub f: Option<BuildinFunc>,
	pub param:*const Code,
}

// #[derive(Debug,Clone,Copy,PartialEq)]
// pub enum Code{
// 	Buildin(Buildin),
// 	Derived(*const Code)
// }



impl Code{
	pub fn basic(f:BuildinFunc,v:isize)->Self{
		Code{f:Some(f),param: v as *const Code}

	}
	pub fn word(c:&[Code])->Self{
		Code{f:None,param:c as *const [_] as *const _}
	}
	pub fn word_raw(param:*const Code)->Self{
		Code{f:None,param}
	}

	pub fn is_null(&self)->bool{
		self.f.is_none() && self.param.is_null()
	}
}

pub struct VmEasyMemory<const STACK_SIZE : usize> {
	param:[MaybeUninit<*mut PalData>;STACK_SIZE] ,
	data:[MaybeUninit<PalData>;STACK_SIZE],
	types:[MaybeUninit<PalData>;STACK_SIZE],
}

impl<const STACK_SIZE: usize > Default for VmEasyMemory<STACK_SIZE>{

fn default() -> Self {
	Self{
		param:make_storage(),
		data:make_storage(),
		types:make_storage(),
	}
}
}

impl<const STACK_SIZE: usize> VmEasyMemory<STACK_SIZE>{
	pub fn new()->Self{
		Self::default()

	}

	pub fn make_vm(&mut self) -> Vm{
		Vm{
			param_stack:StackRef::from_slice(&mut self.param),
			data_stack:StackRef::from_slice(&mut self.data),
			type_stack:StackRef::from_slice(&mut self.types),
		}
	}
}

pub struct Vm<'a> {
	pub param_stack:StackRef<'a, *mut PalData> ,
	pub data_stack:StackRef<'a, PalData>,
	pub type_stack:StackRef<'a, PalData>,
	// pub struct 
}

impl Vm<'_> {
	/// # Safety
	/// the pointer past must point to valid code
	/// the stacks must contain the correct inputs
	pub unsafe fn excute_code(&mut self,code:*const Code) -> *const Code{
		unsafe{
			match (*code).f {
				Some(x) => (x)(code,self),
				None => {
					let mut code = (*code).param;
					loop {
						code = self.excute_code(code);
						if code.is_null(){
							return code;
						}
						//anoyingly some jumps may be 1 below the allocation so we need this
						code = code.wrapping_add(1)
					}
				}
			}
		}
	}
}
