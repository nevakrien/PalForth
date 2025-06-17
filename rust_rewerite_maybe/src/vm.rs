use std::ptr;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicPtr;
use crate::stack::make_storage;
use std::mem::MaybeUninit;
use crate::PalData;
use crate::stack::StackRef;
use core::mem::transmute;

pub type BuildinFunc =  for<'vm> unsafe extern "C-unwind" fn(*const Code,&mut Vm<'vm>) -> *const Code;


#[derive(Debug)]
pub struct BuildinPtr{
	inner:AtomicPtr<()>
}
impl BuildinPtr{
	#[inline]
	pub fn new(f:BuildinFunc)->Self{
		Self{
			inner:AtomicPtr::new(unsafe{transmute(f)})
		}
	}

	#[inline]
	pub fn empty() -> Self{
		Self{
			inner:AtomicPtr::new(ptr::null_mut())
		}
	}

	#[inline(always)]
	pub unsafe fn call(&self,code:*const Code,vm:&mut Vm)-> *const Code{unsafe{
		let f = self.load(Ordering::Relaxed).unwrap_unchecked();
		f(code, vm)
	}}

	#[inline(always)]
	pub fn load(&self,order:Ordering)-> Option<BuildinFunc> {unsafe{
		transmute(self.inner.load(order))
	}}

	// #[inline(always)]
	// pub unsafe fn store(&self,f:BuildinFunc,order:Ordering,) {unsafe{
	// 	let x :*mut () = core::mem::transmute(self.inner.load(order))
	// }}
}

#[repr(C,align(8))]
#[derive(Debug)]
pub struct Code{
	pub f: BuildinPtr,
	pub param:*const Code,
}

// #[derive(Debug,Clone,Copy,PartialEq)]
// pub enum Code{
// 	Buildin(Buildin),
// 	Derived(*const Code)
// }



impl Code{

	#[inline]
	pub fn basic(f:BuildinFunc,v:isize)->Self{
		Code{f:BuildinPtr::new(f),param: v as *const Code}

	}

	#[inline]
	pub fn word(c:&[Code])->Self{
		Code{f:BuildinPtr::empty(),param:c as *const [_] as *const _}
	}

	#[inline]
	pub fn word_raw(param:*const Code)->Self{
		Code{f:BuildinPtr::empty(),param}
	}

	#[inline]
	pub fn is_null(&self)->bool{
		self.f.load(Ordering::Relaxed).is_none() && self.param.is_null()
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
			match (*code).f.load(Ordering::Relaxed) {
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
