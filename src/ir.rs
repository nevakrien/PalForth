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

    pub fn simple_add_word(&mut self, word: &Word<'lex>) -> Result<(), SigError<'lex>> {
        word.runtime.check_sig(&mut self.stack)?;
        //add the word into the function
        self.lex
            .code_mem
            .save(Code::word_raw(word.runtime.exe.get()))
            .expect("out of code mem");
        Ok(())
    }

    ///verifies the stack is empty and returns the generated code
    #[inline]
    pub fn finalize_code(self) -> Result<&'lex [Code],SigStack<'me, 'lex>>{
    	if self.stack.stack.is_empty(){
        	Ok(self.lex.code_mem.index_checkpoint(self.start))
    	}else{
    		Err(self.stack)
    	}
    }
}

pub struct Word<'lex> {
    pub name: &'lex str,
    pub runtime: RuntimeCode<'lex>,
    pub immidate: Option<&'lex UnsafeCell<Code>>,
}

pub struct RuntimeCode<'lex> {
    exe: &'lex UnsafeCell<Code>,
    pub input_sig: &'lex [SigItem<'lex>],
    pub output_sig: &'lex [SigItem<'lex>],
}

impl From<RuntimeCode<'_>> for *const Code {
    fn from(x: RuntimeCode<'_>) -> Self {
        x.exe.get()
    }
}

impl<'lex> RuntimeCode<'lex> {
    ///# Safety
    /// same as [`Vm::execute_code`]
    #[inline(always)]
    pub unsafe fn run(&self, vm: &mut Vm) {
        unsafe { vm.execute_code(self.exe.get()) }
    }

    ///# Safety
    ///the type stack must hold correct information
    ///other than that checks handle everything
    #[inline]
    pub unsafe fn comp_run_checked(&self, vm: &mut Vm<'_, 'lex>) -> Result<(), SigError<'lex>> {
        #[rustfmt::skip]
        let comp = vm.comp.as_mut()
        .expect("need compile time context to run immidate");

        self.check_sig(&mut comp.immidate_stack)?;

        unsafe { self.run(vm) };

        Ok(())
    }

    #[inline]
    fn check_sig(&self, sig: &mut SigStack<'_, 'lex>) -> Result<(), SigError<'lex>> {
        sig.call_sig(self.output_sig, self.input_sig)
    }
}
