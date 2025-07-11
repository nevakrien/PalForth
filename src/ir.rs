use crate::buildins::ret;
use crate::Code;
use crate::input::InputStream;
use crate::lex::Lex;
use crate::lex::StackAllocator;
use crate::types::SigError;
use crate::types::SigItem;
use crate::types::SigStack;
use crate::vm::Vm;

pub struct CompContext<'me, 'lex> {
    pub lex: &'me mut Lex<'lex>,
    start: usize,
    pub stack: SigStack<'lex>,
    pub immidate_stack: SigStack<'lex>,
    pub input: Option<&'me mut dyn InputStream>,
}

impl<'me, 'lex> CompContext<'me, 'lex> {
    pub fn new(lex: &'me mut Lex<'lex>, input: Option<&'me mut dyn InputStream>) -> Self {
        Self {
            start: lex.code_mem.check_point(),
            lex,
            stack: SigStack::new(),
            immidate_stack: SigStack::new(),
            input,
        }
    }

    pub fn add_runtime_code(&mut self, runtime: &RuntimeCode<'lex>) -> Result<(), SigError<'lex>> {
        runtime.check_sig(&mut self.stack)?;
        runtime.save_to_alloc(&mut self.lex.code_mem);
        Ok(())
    }

    ///verifies the stack is empty and returns the generated code
    #[inline]
    pub fn finalize_code(&self) -> Result<&'lex [Code], ()> {
        if self.stack.verify_end() {
            unsafe {Ok(self.lex.code_mem.index_checkpoint(self.start))}
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
        self.lex
            .code_mem
            .save(Code::basic(ret, 0))
            .expect("out of code mem");

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

#[derive(Debug, Clone)]
pub struct Word<'lex> {
    pub name: &'lex str,
    pub runtime: RuntimeCode<'lex>,
    pub immidate: Option<&'lex Code>,
}

///a moveble peice of code that may or may not be inlined
///for the most part inlined code should be reserved for buildins
///inlining derived words can be good but it requires the JIT to do double work
#[derive(Debug, Clone)]
pub enum Exe<'lex> {
    ///contains the code AND a ret terminator in the end
    Inlined(&'lex [Code]),
    Outlined(&'lex [Code]),
}

impl<'lex> Exe<'lex> {
    pub fn inner_slice(&self) -> &'lex [Code] {
        match self {
            Exe::Inlined(s) | Exe::Outlined(s) => s,
        }
    }

    pub fn code_ptr(&self) -> *const Code {
        self.inner_slice().as_ptr()
    }

    pub fn as_word(&self) -> Code {
        Code::word_raw(self.code_ptr())
    }

    pub fn save_to_alloc(&self, alloc: &mut StackAllocator<Code>) {
        match self {
            Exe::Outlined(slice) => {
                alloc.save(Code::word(slice)).expect("out of code mem");
            }
            Exe::Inlined(slice) => {
                for c in slice[..slice.len() - 1].iter() {
                    alloc.save(c.shallow_clone()).expect("out of code mem");
                }
            }
        };
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeCode<'lex> {
    pub exe: Exe<'lex>,
    pub input_sig: &'lex [SigItem<'lex>],
    pub output_sig: &'lex [SigItem<'lex>],
}

impl<'lex> RuntimeCode<'lex> {
    ///# Safety
    /// same as [`Vm::execute_code`]
    #[inline(always)]
    pub unsafe fn run(&self, vm: &mut Vm) {
        unsafe { vm.execute_code(&self.exe.as_word()) }
    }

    pub fn save_to_alloc(&self, alloc: &mut StackAllocator<Code>) {
        self.exe.save_to_alloc(alloc)
    }

    ///# Safety
    ///the type stack must hold correct information
    ///other than that checks handle everything
    #[inline]
    pub unsafe fn comp_run_checked(
        &self,
        vm: &mut Vm<'_, 'lex, '_>,
    ) -> Result<(), SigError<'lex>> {
        let comp = vm.comp.get_comp_crash();

        self.check_sig(&mut comp.immidate_stack)?;

        unsafe { self.run(vm) };

        Ok(())
    }

    #[inline]
    fn check_sig(&self, sig: &mut SigStack<'lex>) -> Result<(), SigError<'lex>> {
        sig.call_sig(self.output_sig, self.input_sig)
    }
}
