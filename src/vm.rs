use crate::PalData;
use crate::buildins::unwrap_over;
use crate::buildins::unwrap_under;
use crate::ir::CompContext;
use crate::stack::StackRef;
use crate::stack::make_storage;
use core::mem::MaybeUninit;
use core::mem::transmute;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;

pub type BuildinFunc =
    for<'vm> unsafe extern "C-unwind" fn(*const Code, &mut Vm<'vm, '_>) -> *const Code;

#[derive(Debug)]
pub struct BuildinPtr {
    inner: AtomicPtr<()>,
}
impl BuildinPtr {
    #[inline]
    pub fn new(f: BuildinFunc) -> Self {
        Self {
            inner: AtomicPtr::new(f as *mut ()),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// # Safety
    /// this calls the underlying function so all its conditons apply
    /// more over the memory housing the code must be in cache
    /// this means the function must have been written with Relese atomic ordering
    #[inline(always)]
    pub unsafe fn call(&self, code: *const Code, vm: &mut Vm) -> *const Code {
        unsafe {
            let f = self.load(Ordering::Relaxed).unwrap_unchecked();
            f(code, vm)
        }
    }

    #[inline(always)]
    pub fn load(&self, order: Ordering) -> Option<BuildinFunc> {
        unsafe { transmute(self.inner.load(order)) }
    }

    // #[inline(always)]
    // pub unsafe fn store(&self,f:BuildinFunc,order:Ordering,) {unsafe{
    // 	let x :*mut () = core::mem::transmute(self.inner.load(order))
    // }}
}

#[repr(C, align(8))]
#[derive(Debug)]
pub struct Code {
    //so both of these are atomic but dont touch them togehter
    //in general only one of this is acessed atomic at any moment
    pub f: BuildinPtr,
    pub param: AtomicPtr<Code>, //this is const
}

// #[derive(Debug,Clone,Copy,PartialEq)]
// pub enum Code{
// 	Buildin(Buildin),
// 	Derived(*const Code)
// }

impl Code {
    #[inline]
    pub fn basic(f: BuildinFunc, v: isize) -> Self {
        Code {
            f: BuildinPtr::new(f),
            param: AtomicPtr::new(v as *mut Code),
        }
    }

    #[inline]
    pub fn basic_raw(f: BuildinFunc, p: *const Code) -> Self {
        Code {
            f: BuildinPtr::new(f),
            param: AtomicPtr::new(p as *mut _),
        }
    }

    #[inline]
    pub fn word(c: &[Code]) -> Self {
        Code {
            f: BuildinPtr::empty(),
            param: AtomicPtr::new(c as *const [_] as *const _ as *mut _),
        }
    }

    #[inline]
    pub fn word_raw(param: *const Code) -> Self {
        Code {
            f: BuildinPtr::empty(),
            param: AtomicPtr::new(param as *mut _),
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.f.load(Ordering::Relaxed).is_none() && self.param.load(Ordering::Relaxed).is_null()
    }

    #[inline(always)]
    pub fn shallow_clone(&self)->Self{
    	match self.f.load(Ordering::Relaxed){
    		Some(f)=>Code::basic_raw(f,self.param.load(Ordering::Relaxed)),
    		None=>Code::word_raw(self.param.load(Ordering::Relaxed))
    	}
    }
}

pub struct VmEasyMemory<const STACK_SIZE: usize> {
    param: [MaybeUninit<*mut PalData>; STACK_SIZE],
    data: [MaybeUninit<PalData>; STACK_SIZE],
    rs: [MaybeUninit<*const Code>; STACK_SIZE],
}

impl<const STACK_SIZE: usize> Default for VmEasyMemory<STACK_SIZE> {
    fn default() -> Self {
        Self {
            param: make_storage(),
            data: make_storage(),
            rs: make_storage(),
        }
    }
}

impl<'lex, const STACK_SIZE: usize> VmEasyMemory<STACK_SIZE> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make_vm(&mut self) -> Vm<'_, '_> {
        Vm {
            param_stack: StackRef::from_slice(&mut self.param),
            data_stack: StackRef::from_slice(&mut self.data),
            return_stack: StackRef::from_slice(&mut self.rs),
            comp: None,
        }
    }
}

pub struct Vm<'a, 'lex> {
    pub param_stack: StackRef<'a, *mut PalData>,
    pub data_stack: StackRef<'a, PalData>,
    pub return_stack: StackRef<'a, *const Code>,

    pub comp: Option<&'a mut CompContext<'a, 'lex>>,
}
impl Vm<'_, '_> {
    /// # Safety
    /// the code must be safe to execute in a threaded way (ie no use of return stack for control flow)
    /// the pointer past must point to valid code
    /// the stacks must contain the correct inputs
    pub unsafe fn execute_threaded(&mut self, code: *const Code) -> *const Code {
        unsafe {
            match (*code).f.load(Ordering::Relaxed) {
                Some(x) => (x)(code, self),
                None => {
                    let mut code = (*code).param.load(Ordering::Relaxed) as *const _;
                    loop {
                        code = self.execute_threaded(code);
                        if code.is_null() {
                            return code;
                        }
                        //anoyingly some jumps may be 1 below the allocation so we need this
                        code = code.wrapping_add(1)
                    }
                }
            }
        }
    }

    /// # Safety
    /// the pointer past must point to valid code
    /// the stacks must contain the correct inputs
    pub unsafe fn execute_code(&mut self, mut code: *const Code) {
        unsafe {
            //compiler can load the return stack
            loop {
                //first get a primitive and run it
                let mut primitive = (*code).f.load(Ordering::Relaxed);
                while primitive.is_none() {
                    unwrap_over(self.return_stack.push(code).ok());
                    code = (*code).param.load(Ordering::Relaxed) as *const _;
                    primitive = (*code).f.load(Ordering::Relaxed);
                }

                code = primitive.unwrap_unchecked()(code, self);

                //compiler must unload the return stack since we just called &mut Vm
                //it will now (likely) load it again since no calls with &mut Vm are made after
                //if we were to commit to optimizing this loop the trick would be to use head pointer directly
                //however this means a VM cant switch its stack which means multi tasking cant works from within
                //its better to allow out right switching the return stack as an internal

                //is there more code to run?
                if code.is_null() {
                    //if this is the outer frame then code+1 is junk
                    //and we need to return now
                    //also if the return stack is empty
                    if self.return_stack.len() <= 1 {
                        unwrap_under(self.return_stack.pop());
                        return;
                    }

                    //this bit was rewritten as LLVM made sub par assembly
                    code = self.return_stack.pop().unwrap_unchecked();
                }
                code = code.wrapping_add(1); //add is the same whether or not you are an internal
            }
        }
    }
}

// 	/// # Safety
// 	/// the pointer must point to valid code
//     #[inline(always)]
//     pub unsafe fn descend_to_primitive(&mut self, mut ip: *const Code) -> *const Code { unsafe {
//         use core::sync::atomic::Ordering::Relaxed;

//         #[cfg(feature = "trace_vm")]
//         println!("decending {ip:?}");

//         // Walk through "colon" headers, stacking each one.
//         let mut f = (*ip).f.load(Relaxed);
//         while f.is_none() {
//             // Push the *header* (not header+1 — consistent with your scheme)
//             self.return_stack.push(ip).unwrap();
//             ip = (*ip).param.load(Relaxed) as *const Code;

//             #[cfg(feature = "trace_vm")]
//        		println!("found {ip:?} in decent");

//             f  = (*ip).f.load(Relaxed);
//         }
//         ip // guaranteed primitive cell
//     }}

//     /// # Safety
// 	/// the pointer past must point to valid code
// 	/// the stacks must contain the correct inputs
//     pub unsafe fn execute_code(&mut self, mut ip: *const Code) {
//         #[cfg(feature = "trace_vm")]
// 	    println!("starting execute_code with {ip:?}");

//         loop {
//             /* 1. descend, then run the primitive */
//             unsafe {
//                 ip = self.descend_to_primitive(ip);
//                 let prim = (*ip).f.load(Ordering::Relaxed).unwrap_unchecked();
//                 ip = prim(ip, self);
//             }

//             /* 2. finished a word?  pop caller or return */
//             if ip.is_null() {
//                 if self.return_stack.len() <= 1 {
//                     self.return_stack.pop().unwrap();
//                     return;
//                 }
//                 ip = unsafe{self.return_stack.pop().unwrap_unchecked()};

//                 #[cfg(feature = "trace_vm")]{
//                 	let return_target = ip.wrapping_add(1);
// 	        		println!("returning into {return_target:?}");
// 	        	}
//             }

//             /* 3. step to next threaded cell */
//             ip = ip.wrapping_add(1);
//         }
//     }
// }
