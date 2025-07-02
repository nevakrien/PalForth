use crate::Code;
use crate::lex::Lex;
use crate::types::SigError;
use crate::types::SigItem;
use crate::types::SigStack;
use crate::vm::Vm;
use core::cell::UnsafeCell;

pub struct CompContext<'me, 'lex> {
    pub lex: &'me mut Lex<'lex>,
    pub stack: SigStack<'me, 'lex>,
    pub immidate_stack: SigStack<'me, 'lex>,
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
