use crate::lex::DelayedSlice;
use crate::lex::Lex;
use crate::lex::StackAllocator;
use crate::lex::StackWriter;
use crate::stack::StackRef;
use crate::stack::make_storage;
use core::cell::Cell;
use core::cell::RefCell;
use core::fmt;
use core::fmt::Write;
use core::mem::MaybeUninit;

pub const READ_FLAG: u8 = 0x1;
pub const WRITE_FLAG: u8 = 0x2;
pub const UNIQUE_FLAG: u8 = 0x4;
pub const OUTPUT_FLAG: u8 = 0x8; //whethere or not this stays on the output stack
pub const RAW_FLAG: u8 = 0x10; //if set the value is passed on the data stack (this convention cant be easily automated)
pub const INDEX_FLAG: u8 = 0x20; //only relvent for outputs if set the return pointer may be ANY pointer Derived!!! from the input (which has lifetime implications) this convention cant be easily automated
pub type RwT = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Boxed,
    Indexed,
    Raw,
    Invalid,
}

impl AccessMode {
    pub fn from_bits(bits: RwT) -> Self {
        let raw = bits & RAW_FLAG != 0;
        let idx = bits & INDEX_FLAG != 0;
        match (raw, idx) {
            (false, false) => AccessMode::Boxed,
            (false, true) => AccessMode::Indexed,
            (true, false) => AccessMode::Raw,
            (true, true) => AccessMode::Invalid,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AccessMode::Boxed => "boxed",
            AccessMode::Indexed => "indexed",
            AccessMode::Raw => "raw",
            AccessMode::Invalid => "invalid",
        }
    }
}

pub enum SigError<'lex> {
    WrongType {
        found: &'lex Type<'lex>,
        wanted: &'lex Type<'lex>,
    },
    NeedsUnique,
    AlreadyBorrowed,
    BasicSigError {
        clash: RwT,
        have: RwT,
    }, // cleaner and clearer
    MissingArgument(SigItem<'lex>),
}

impl fmt::Display for SigError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigError::WrongType { found, wanted } => {
                writeln!(f, "  - expected {}, but got {}", wanted.name, found.name)
            }
            SigError::NeedsUnique => write!(
                f,
                "Cannot borrow: value is already borrowed (requires unique access)"
            ),
            SigError::AlreadyBorrowed => {
                write!(f, "Cannot borrow: value is currently borrowed as unique")
            }
            SigError::BasicSigError { clash, have } => {
                writeln!(f, "Signature mismatch:")?;

                if clash & READ_FLAG != 0 {
                    writeln!(f, "  - expected read access, but it's missing")?;
                }
                if clash & WRITE_FLAG != 0 {
                    writeln!(f, "  - expected write access, but it's missing")?;
                }
                if clash & UNIQUE_FLAG != 0 {
                    writeln!(f, "  - expected unique access, but it's missing")?;
                }
                if clash & OUTPUT_FLAG != 0 {
                    let actual = if have & OUTPUT_FLAG != 0 {
                        "output"
                    } else {
                        "input"
                    };
                    let expected = if have & OUTPUT_FLAG != 0 {
                        "input"
                    } else {
                        "output"
                    };
                    writeln!(f, "  - expected {}, but got {}", expected, actual)?;
                }

                if (clash & (RAW_FLAG | INDEX_FLAG)) != 0 {
                    let expected = AccessMode::from_bits(*clash);
                    let actual = AccessMode::from_bits(*have);

                    // Skip if AccessMode::Invalid – might be a programming error
                    if expected != actual {
                        writeln!(
                            f,
                            "  - expected {}, but got {}",
                            expected.name(),
                            actual.name()
                        )?;
                    }
                }

                Ok(())
            }
            SigError::MissingArgument(a) => write!(f, "Missing an argument of type {a}"),
        }
    }
}

impl fmt::Debug for SigError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SigError(\"{self}\")")
    }
}

pub type TypeP<'lex> = &'lex Type<'lex>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type<'lex> {
    pub inner: TypeInner<'lex>,
    pub size: i32,
    pub cells: i32,
    pub name: &'lex str,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum TypeInner<'lex> {
    Basic,
    Alias(TypeP<'lex>, &'lex str),
    Array(TypeP<'lex>, Option<i32>),
    Cluster(DelayedSlice<'lex, TypeP<'lex>>),
}

impl<'lex> TypeInner<'lex> {
    pub fn get_type_ref(&self, lex: &mut Lex<'lex>) -> TypeP<'lex> {
        if let Some(x) = lex.type_map.get(self) {
            return x;
        }

        let (name, cells, size) = match self {
            TypeInner::Basic => unreachable!("missing basic type in the table"),
            TypeInner::Alias(parent, name) => (*name, parent.cells, parent.size),
            TypeInner::Array(elem, num) => {
                let mut writer = StackWriter::new(&mut lex.comp_data_mem);
                match num {
                    None => {
                        write!(writer, "Array({})", elem.name).expect("Out of memory in comp data");
                        let (cells, size) = (2, 2 * size_of::<*const ()>());
                        (&*writer.finish(), cells, size as i32)
                    }
                    Some(len) => {
                        write!(writer, "Array<{}>({})", len, elem.name)
                            .expect("Out of memory in comp data");
                        (&*writer.finish(), len * elem.cells, len * elem.size)
                    }
                }
            }
            TypeInner::Cluster(elems) => {
                let mut writer = StackWriter::new(&mut lex.comp_data_mem);
                let mut cells = 0;
                let mut size = 0;
                write!(writer, "Cluster(").expect("Out of memory in comp data");
                for (i, elem) in elems.iter().enumerate() {
                    cells += elem.cells;
                    size += elem.size;
                    if i > 0 {
                        write!(writer, ", ").expect("Out of memory in comp data");
                    }
                    write!(writer, "{}", elem.name).expect("Out of memory in comp data");
                }
                write!(writer, ")").expect("Out of memory in comp data");
                (writer.finish() as &_, cells, size)
            }
        };
        let me = lex
            .types_mem
            .save(Type {
                inner: *self,
                name,
                cells,
                size,
            })
            .expect("Out of memory in types arena");

        if lex.type_map.insert(&me.inner, me).is_some() {
            //This should be unreachable because of the check at the start of the function
            unreachable!();
        }
        me
    }
}

/*──────────────────  SIGNATURES ────────────────── */
//signatures are allways of this form
//[outputs] [inputs]
//
//inputs are consumed out of the stack while outputs remain
//in PALFORTH virtually all outputs are done by injection
//meaning a pointer to the output spot is passed to the function the output is written to it and then it remains on the stack

#[derive(Debug, Clone, Copy)]
pub struct SigItem<'lex> {
    pub tp: &'lex Type<'lex>,
    pub permissions: RwT,
}
impl fmt::Display for SigItem<'_> {
    #[allow(unused_assignments)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [", self.tp.name)?;

        let mut first = true;
        macro_rules! show_flag {
            ($flag:ident, $label:expr) => {
                if self.permissions & $flag != 0 {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, $label)?;
                    first = false;
                }
            };
        }

        show_flag!(READ_FLAG, "read");
        show_flag!(WRITE_FLAG, "write");
        show_flag!(UNIQUE_FLAG, "unique");
        show_flag!(OUTPUT_FLAG, "output");
        show_flag!(RAW_FLAG, "raw");
        show_flag!(INDEX_FLAG, "index");

        write!(f, "]")
    }
}

#[derive(Debug)]
pub struct CompVar<'ctx, 'lex> {
    pub tp: &'lex Type<'lex>,
    pub offset_from_start: i32,        // first local is 0
    pub num_borrowed: &'ctx Cell<i32>, // unique borrow is -1
    pub permissions: RwT,
}

fn check_subset(have: RwT, sig: RwT) -> Result<(), SigError<'static>> {
    let mut clash = 0;

    if (sig & UNIQUE_FLAG != 0) && (have & UNIQUE_FLAG == 0) {
        clash |= UNIQUE_FLAG;
    }
    if (sig & WRITE_FLAG != 0) && (have & WRITE_FLAG == 0) {
        clash |= WRITE_FLAG;
    }
    if (sig & READ_FLAG != 0) && (have & READ_FLAG == 0) {
        clash |= READ_FLAG;
    }
    if (sig & OUTPUT_FLAG) != (have & OUTPUT_FLAG) {
        clash |= OUTPUT_FLAG;
    }

    if (sig & RAW_FLAG) != (have & RAW_FLAG) {
        clash |= RAW_FLAG;
    }

    if (sig & INDEX_FLAG) != (have & INDEX_FLAG) {
        clash |= INDEX_FLAG;
    }

    if clash != 0 {
        Err(SigError::BasicSigError { clash, have })
    } else {
        Ok(())
    }
}

pub fn use_box_as<'lex>(
    box_var: &mut CompVar<'_, 'lex>,
    sig: &SigItem<'lex>,
) -> Result<(), SigError<'lex>> {
    if box_var.tp as *const _ != sig.tp as *const _ {
        return Err(SigError::WrongType {
            found: box_var.tp,
            wanted: sig.tp,
        });
    }
    check_subset(box_var.permissions, sig.permissions)?;

    if box_var.num_borrowed.get() == -1 {
        return Err(SigError::AlreadyBorrowed);
    }
    if (sig.permissions & UNIQUE_FLAG != 0) && box_var.num_borrowed.get() != 0 {
        return Err(SigError::NeedsUnique);
    }

    if sig.permissions & UNIQUE_FLAG != 0 {
        box_var.num_borrowed.set(-1);
    } else {
        //num_borrowed++
        box_var.num_borrowed.set(box_var.num_borrowed.get() + 1);
    }

    Ok(())
}

pub fn free_box_use(box_var: &mut CompVar, sig: RwT) {
    if sig & UNIQUE_FLAG != 0 {
        box_var.num_borrowed.set(0);
    } else {
        //num_borrowed--
        box_var.num_borrowed.set(box_var.num_borrowed.get() - 1);
    }
}

/// # Safety
/// changing any of the underlying stacks is considered unsound
pub struct SigStack<'me, 'lex> {
    cells_locals: i32,
    var_arena: StackAllocator<'me, RefCell<CompVar<'me, 'lex>>>,
    borrows_arena: StackAllocator<'me, Cell<i32>>,
    pub stack: StackRef<'me, &'me RefCell<CompVar<'me, 'lex>>>,
}

impl<'me, 'lex> SigStack<'me, 'lex> {
    pub fn add_local(&mut self, tp: &'lex Type<'lex>) -> &'me RefCell<CompVar<'me, 'lex>> {
        let num_borrowed = self
            .borrows_arena
            .save(Cell::new(0))
            .expect("overflow borrow arena");

        let var = CompVar {
            tp,
            permissions: READ_FLAG | WRITE_FLAG | UNIQUE_FLAG,
            num_borrowed,
            offset_from_start: self.cells_locals,
        };
        let ans = self.var_arena.save(var.into()).expect("overflow var arena");
        self.cells_locals += tp.cells;
        ans
    }

    pub fn add_borrows(&mut self, num: i32) -> &'me Cell<i32> {
        self.borrows_arena
            .save(Cell::new(num))
            .expect("overflow borrow arena")
    }

    ///checks a signature and pops out the inputs from the argument stack
    ///on faliure the stack is left in a weird but safe state
    pub fn call_sig(
        &mut self,
        outputs: &[SigItem<'lex>],
        inputs: &[SigItem<'lex>],
    ) -> Result<(), SigError<'lex>> {
        for t in inputs.iter().rev() {
            match self.stack.pop() {
                None => return Err(SigError::MissingArgument(*t)),
                Some(b) => use_box_as(&mut b.borrow_mut(), t)?,
            };
        }

        //checkpoint here so outputs arent poped
        let mut stack = StackRef::new_full(self.stack.split().0);

        for t in outputs.iter().rev() {
            match stack.pop() {
                None => return Err(SigError::MissingArgument(*t)),
                Some(b) => use_box_as(&mut b.borrow_mut(), t)?,
            };
        }

        Ok(())
    }
}

// Easy memory struct
pub struct SigStackEasyMemory<'me, 'lex, const STACK_SIZE: usize> {
    var_arena_mem: [MaybeUninit<RefCell<CompVar<'me, 'lex>>>; STACK_SIZE],
    borrows_arena_mem: [MaybeUninit<Cell<i32>>; STACK_SIZE],
    stack_mem: [MaybeUninit<&'me RefCell<CompVar<'me, 'lex>>>; STACK_SIZE],
}

impl<const STACK_SIZE: usize> Default for SigStackEasyMemory<'_, '_, STACK_SIZE> {
    fn default() -> Self {
        Self {
            var_arena_mem: make_storage(),
            borrows_arena_mem: make_storage(),
            stack_mem: make_storage(),
        }
    }
}

impl<'me, 'lex, const STACK_SIZE: usize> SigStackEasyMemory<'me, 'lex, STACK_SIZE> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make_sig_stack(&'me mut self) -> SigStack<'me, 'lex> {
        SigStack {
            cells_locals: 0,
            var_arena: StackAllocator::new(&mut self.var_arena_mem),
            borrows_arena: StackAllocator::new(&mut self.borrows_arena_mem),
            stack: StackRef::from_slice(&mut self.stack_mem),
        }
    }
}

/* ───────────────────────── SIGSTACK TYPECHECK ───────────────────────── */

#[cfg(test)]
fn make_types() -> (Type<'static>, Type<'static>) {
    (
        Type {
            inner: TypeInner::Basic,
            size: 4,
            cells: 1,
            name: "int",
        },
        Type {
            inner: TypeInner::Basic,
            size: 4,
            cells: 1,
            name: "float",
        },
    )
}

#[test]
fn sig_stack_success_case() {
    let (type_int, type_float) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let var1 = sig_stack.add_local(&type_int);

    let num_borrowed = sig_stack.add_borrows(0);
    let var2 = sig_stack
        .var_arena
        .save(RefCell::new(CompVar {
            tp: &type_float,
            offset_from_start: 1,
            num_borrowed,
            permissions: READ_FLAG,
        }))
        .unwrap();
    sig_stack.stack.push(var1).unwrap();
    sig_stack.stack.push(var2).unwrap();

    let inputs = [
        SigItem {
            tp: &type_int,
            permissions: READ_FLAG,
        },
        SigItem {
            tp: &type_float,
            permissions: READ_FLAG,
        },
    ];
    sig_stack.call_sig(&[], &inputs).unwrap();
    assert_eq!(sig_stack.stack.len(), 0);
}

#[test]
fn sig_stack_wrong_type_error() {
    let (type_int, type_float) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let num_borrowed = sig_stack.add_borrows(0);
    let var1 = sig_stack
        .var_arena
        .save(RefCell::new(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            num_borrowed,
            permissions: READ_FLAG | WRITE_FLAG | UNIQUE_FLAG,
        }))
        .unwrap();
    sig_stack.stack.push(var1).unwrap();

    let inputs = [SigItem {
        tp: &type_float,
        permissions: READ_FLAG,
    }];
    let err = sig_stack.call_sig(&[], &inputs).unwrap_err();
    match err {
        SigError::WrongType { found, wanted } => {
            assert_eq!(found.name, "int");
            assert_eq!(wanted.name, "float");
        }
        _ => panic!("Expected WrongType error"),
    }
}

#[test]
fn sig_stack_missing_argument_error() {
    let (type_int, _) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let inputs = [SigItem {
        tp: &type_int,
        permissions: READ_FLAG,
    }];
    let err = sig_stack.call_sig(&[], &inputs).unwrap_err();
    match err {
        SigError::MissingArgument(item) => {
            assert_eq!(item.tp.name, "int");
        }
        _ => panic!("Expected MissingArgument error"),
    }
}

#[test]
fn sig_stack_permission_error() {
    let (type_int, _) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let num_borrowed = sig_stack.add_borrows(0);
    let var1 = sig_stack
        .var_arena
        .save(RefCell::new(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            num_borrowed,
            permissions: READ_FLAG,
        }))
        .unwrap();
    sig_stack.stack.push(var1).unwrap();

    let inputs = [SigItem {
        tp: &type_int,
        permissions: WRITE_FLAG,
    }];
    let err = sig_stack.call_sig(&[], &inputs).unwrap_err();
    match err {
        SigError::BasicSigError { clash, have } => {
            assert_ne!(clash & WRITE_FLAG, 0);
            assert_eq!(have & WRITE_FLAG, 0);
        }
        _ => panic!("Expected BasicSigError for permissions"),
    }
}

#[test]
fn sig_stack_needs_unique_error() {
    let (type_int, _) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let num_borrowed = sig_stack.add_borrows(1);
    let var1 = sig_stack
        .var_arena
        .save(RefCell::new(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            num_borrowed,
            permissions: READ_FLAG | UNIQUE_FLAG,
        }))
        .unwrap();
    sig_stack.stack.push(var1).unwrap();
    sig_stack.stack.push(var1).unwrap();

    let inputs = [
        SigItem {
            tp: &type_int,
            permissions: UNIQUE_FLAG,
        },
        SigItem {
            tp: &type_int,
            permissions: UNIQUE_FLAG,
        },
    ];
    let err = sig_stack.call_sig(&[], &inputs).unwrap_err();
    assert!(matches!(err, SigError::NeedsUnique));
}

#[test]
fn sig_stack_already_borrowed_error() {
    let (type_int, _) = make_types();
    let mut sig_mem = SigStackEasyMemory::<'_, '_, 1024>::new();
    let mut sig_stack = sig_mem.make_sig_stack();

    let num_borrowed = sig_stack.add_borrows(-1);
    let var1 = sig_stack
        .var_arena
        .save(RefCell::new(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            num_borrowed,
            permissions: READ_FLAG | UNIQUE_FLAG,
        }))
        .unwrap();
    sig_stack.stack.push(var1).unwrap();

    let inputs = [SigItem {
        tp: &type_int,
        permissions: READ_FLAG,
    }];
    let err = sig_stack.call_sig(&[], &inputs).unwrap_err();
    assert!(matches!(err, SigError::AlreadyBorrowed));
}
