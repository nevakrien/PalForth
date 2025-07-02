use crate::Code;
use crate::lex::Lex;
use crate::lex::StackAllocatorCheckPoint;
use crate::types::SigError;
use crate::types::SigItem;
use crate::types::SigStack;
use crate::vm::Vm;
use core::cell::UnsafeCell;

pub struct CompContext<'me, 'lex> {
    pub lex: &'me mut Lex<'lex>,
    start: StackAllocatorCheckPoint,
    pub stack: SigStack<'me, 'lex>,
    immidate_stack: SigStack<'me, 'lex>,
}

impl<'me, 'lex> CompContext<'me, 'lex> {
    pub fn new(
        lex: &'me mut Lex<'lex>,
        stack: SigStack<'me, 'lex>,
        immidate_stack: SigStack<'me, 'lex>,
    ) -> Self {
        Self {
            start: lex.code_mem.check_point(),
            lex,
            stack,
            immidate_stack,
        }
    }

    pub fn add_runtime_code(&mut self, runtime: &RuntimeCode<'lex>) -> Result<(), SigError<'lex>> {
        runtime.check_sig(&mut self.stack)?;
        //append the code in
        self.lex
            .code_mem
            .save(runtime.code())
            .expect("out of code mem");
        Ok(())
    }

    ///verifies the stack is empty and returns the generated code
    #[inline]
    pub fn finalize_code(&self) -> Result<&'lex [Code], ()> {
        if self.stack.stack.is_empty() {
            Ok(self.lex.code_mem.index_checkpoint(self.start))
        } else {
            Err(())
        }
    }

    pub fn finalize_and_store_word(
        &mut self,
        name: &'lex str,
        input_sig: &'lex [SigItem<'lex>],
        output_sig: &'lex [SigItem<'lex>],
    ) -> Result<(), ()> {
        let code = self.finalize_code()?;
        let runtime = RuntimeCode {
            exe: Exe::Outlined(code),
            input_sig,
            output_sig,
        };
        let word = Word {
            name,
            runtime,
            immidate: None,
        };
        self.lex.words.insert(name, word);
        Ok(())
    }
}

#[derive(Debug,Clone)]
pub struct Word<'lex> {
    pub name: &'lex str,
    pub runtime: RuntimeCode<'lex>,
    pub immidate: Option<&'lex Code>,
}

///a moveble peice of code that may or may not be inlined
///for the most part inlined code should be reserved for buildins
///inlining derived words can be good but it requires the JIT to do double work
#[derive(Debug)]
pub enum Exe<'lex>{
	Inlined(Code),
	Outlined(&'lex [Code])
}

impl<'lex> Clone for Exe<'lex>{
fn clone(&self) -> Self {
	match self{
		Exe::Outlined(r)=>Exe::Outlined(r),
		Exe::Inlined(code) => Exe::Inlined(code.shallow_clone()),
	}
}
}

impl Exe<'_>{
	pub fn code(self)->Code{
		match self{
			Exe::Inlined(code)=>code,
			Exe::Outlined(slice)=>Code::word(slice),
		}
	}
}

#[derive(Debug,Clone)]
pub struct RuntimeCode<'lex> {
    exe: Exe<'lex>,
    pub input_sig: &'lex [SigItem<'lex>],
    pub output_sig: &'lex [SigItem<'lex>],
}

impl<'lex> RuntimeCode<'lex> {
    ///# Safety
    /// same as [`Vm::execute_code`]
    #[inline(always)]
    pub unsafe fn run(&self, vm: &mut Vm) {
        unsafe { vm.execute_code(&self.code()) }
    }

    pub fn code(&self) -> Code{
    	self.exe.clone().code()
    }

    ///# Safety
    ///the type stack must hold correct information
    ///other than that checks handle everything
    #[inline]
    pub unsafe fn comp_run_checked(&self, vm: &mut Vm<'_, 'lex>) -> Result<(), SigError<'lex>> {
        let comp = vm.comp.get_comp_crash();

        self.check_sig(&mut comp.immidate_stack)?;

        unsafe { self.run(vm) };

        Ok(())
    }

    #[inline]
    fn check_sig(&self, sig: &mut SigStack<'_, 'lex>) -> Result<(), SigError<'lex>> {
        sig.call_sig(self.output_sig, self.input_sig)
    }
}
