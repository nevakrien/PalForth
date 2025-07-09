use alloc::vec::Vec;
use crate::lex::DelayedSlice;
use crate::lex::Lex;
use crate::lex::StackWriter;
use core::fmt;
use core::fmt::Write;


pub type RwT = u8;

///can this be read? (uninilized memory is not marked read)
pub const READ_FLAG: u8 = 0x1;

///can this be written to NOTE that this still does not allow inject which requires [`UNIQUE_FLAG`] as well it only allows for non unique
///this is to avoid bugs where the input to a function also happens to be in the output which is baned by defualt
pub const WRITE_FLAG: u8 = 0x2;

///marks unique access which is expected from outputs by defualt to avoid weird situations of order of writes mattering
///as a bonus this gives us memcpy instead of memove and the ability to make some statments on thread safety
pub const UNIQUE_FLAG: u8 = 0x4;

///whether or not this stays on the output stack
pub const OUTPUT_FLAG: u8 = 0x8;

///if set the value is passed on the data stack (this convention cant be easily automated) 
pub const RAW_FLAG: u8 = 0x10;

///only relvent for outputs if set the return pointer may be ANY pointer Derived!!! from the input (which has lifetime implications) this convention cant be easily automated
pub const INDEX_FLAG: u8 = 0x20;

///whether or not this can be sent to another thread for read (non unique writbles can be made to read while actually not being sync)
///note that to make a sync from a unique write requires a seprate borrow which is unique and does not allow write.
pub const SYNC_FLAG: u8 = 0x40;

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

                if clash & SYNC_FLAG != 0 {
                    writeln!(f, "  - expected multi threaded access, but it's missing")?;
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
        show_flag!(SYNC_FLAG, "sync");

        write!(f, "]")
    }
}

pub type VarId = usize;

#[derive(Debug,Clone)]
pub struct CompVar<'lex> {
    pub tp: &'lex Type<'lex>,
    pub offset_from_start: i32,        // first local is 0
    pub borrow_id: VarId, // unique borrow is -1
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

    if (sig & SYNC_FLAG != 0) && (have & SYNC_FLAG == 0) {
        clash |= SYNC_FLAG;
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
    box_var: &mut CompVar< 'lex>,
    sig: &SigItem<'lex>,
    borrows: &mut Vec<i32>,
) -> Result<(), SigError<'lex>> {
    if box_var.tp as *const _ != sig.tp as *const _ {
        return Err(SigError::WrongType {
            found: box_var.tp,
            wanted: sig.tp,
        });
    }
    check_subset(box_var.permissions, sig.permissions)?;

    if borrows[box_var.borrow_id] == -1 {
        return Err(SigError::AlreadyBorrowed);
    }
    if (sig.permissions & UNIQUE_FLAG != 0) && borrows[box_var.borrow_id] != 0 {
        return Err(SigError::NeedsUnique);
    }

    if sig.permissions & UNIQUE_FLAG != 0 {
        borrows[box_var.borrow_id]=-1;
    } else {
        borrows[box_var.borrow_id]+=1;
    }

    Ok(())
}

pub fn free_box_use(box_var: &mut CompVar, sig: RwT,borrows: &mut Vec<i32>,) {
    if sig & UNIQUE_FLAG != 0 {
        borrows[box_var.borrow_id]=0;
    } else {
        //num_borrowed--
        borrows[box_var.borrow_id]-=1;
    }
}

// pub struct SigStackView<'me,'lex>{
//     cells_locals: i32,
//     var_arena: *mut [CompVar<'lex>],
//     borrows_arena: StackVec<'me, i32>,
//     pub stack: StackRef<'me, VarId>,
// }

/// # Safety
/// changing any of the underlying stacks is considered unsound
#[derive(Clone)]
pub struct SigStack<'lex> {
    cells_locals: i32,
    var_arena: Vec<CompVar<'lex>>,
    borrows_arena: Vec<i32>,
    pub stack: Vec<VarId>,
}

impl Default for SigStack<'_>{

fn default() -> Self { Self{
    cells_locals:0,
    var_arena:Vec::with_capacity(16),
    borrows_arena:Vec::with_capacity(16),
    stack:Vec::with_capacity(32),
} }
}

impl<'lex> SigStack<'lex> {
    pub fn new()->Self{
        Self::default()
    }
    pub fn add_local(&mut self, tp: &'lex Type<'lex>) -> VarId {
        let var = CompVar {
            tp,
            permissions: WRITE_FLAG | READ_FLAG | UNIQUE_FLAG,//not sync since borrow checker would downcast to non unique sync
            borrow_id:self.add_borrows(0),
            offset_from_start: self.cells_locals,
        };
        self.var_arena.push(var.into());
        self.cells_locals += tp.cells;
        self.var_arena.len()-1
    }

    pub fn add_borrows(&mut self, num: i32) -> usize {
        self.borrows_arena.push(num);
        self.borrows_arena.len()-1
    }

    ///checks a signature and pops out the inputs from the argument stack
    ///on faliure the stack is left in a weird but safe state
    pub fn call_sig(
        &mut self,
        outputs: &[SigItem<'lex>],
        inputs: &[SigItem<'lex>],
    ) -> Result<(), SigError<'lex>> {


        //first consume all the inputs and outputs without poping
        let mut stack = self.stack.iter().rev();

        for t in inputs.iter().rev() {
            match stack.next() {
                None => return Err(SigError::MissingArgument(*t)),
                Some(id) => use_box_as(&mut self.var_arena[*id], t,&mut self.borrows_arena)?,
            };
        }

        for t in outputs.iter().rev() {
            match stack.next() {
                None => return Err(SigError::MissingArgument(*t)),
                Some(id) => use_box_as(&mut self.var_arena[*id], t ,&mut self.borrows_arena)?,
            };
        }

        //we have now verified the signature time to free

        for t in inputs.iter().rev() {
            let id = self.stack.pop().unwrap();
            free_box_use(&mut self.var_arena[id], t.permissions,&mut self.borrows_arena);
            
        }

        let mut stack = self.stack.iter().rev();
        for t in outputs.iter().rev() {
            let id = stack.next().unwrap();
            free_box_use(&mut self.var_arena[*id], t.permissions,&mut self.borrows_arena);
        }
        Ok(())
    }

    pub fn is_only_outputs(&self) -> bool{
        for id in self.stack.iter(){
            if (self.var_arena[*id].permissions&OUTPUT_FLAG)==0 {
                return false;
            }
        }

        return true;
    }
}


/* ───────────────────────── SIGSTACK TYPECHECK ───────────────────────── */

/*──────────────────────── test helpers ───────────────────────*/

fn make_int<'lex>() -> Type<'lex> {
    Type { inner: TypeInner::Basic, size: 4, cells: 1, name: "int" }
}
fn make_float<'lex>() -> Type<'lex> {
    Type { inner: TypeInner::Basic, size: 4, cells: 1, name: "float" }
}

/*──────────────────────── success ────────────────────────────*/

#[test]
fn sig_stack_success_case() {
    let (type_int, type_float) = (make_int(), make_float());

    let mut st  = SigStack::new();

    // int variable via helper
    let var1 = st.add_local(&type_int);

    // explicit float variable with custom permissions
    let borrow_id = st.add_borrows(0);
    st.var_arena
        .push(CompVar {
            tp: &type_float,
            offset_from_start: 1,
            borrow_id,
            permissions: READ_FLAG,
        });
    let var2 = st.var_arena.len() - 1;

    st.stack.push(var1);
    st.stack.push(var2);

    let inputs = [
        SigItem { tp: &type_int,   permissions: READ_FLAG },
        SigItem { tp: &type_float, permissions: READ_FLAG },
    ];
    st.call_sig(&[], &inputs).unwrap();
    assert_eq!(st.stack.len(), 0);
}

/*──────────────────────── wrong type ─────────────────────────*/

#[test]
fn sig_stack_wrong_type_error() {
    let (type_int, type_float) = (make_int(), make_float());

    let mut st  = SigStack::new();

    // stack has an `int`
    let var1 = st.add_local(&type_int);
    st.stack.push(var1);

    let inputs = [SigItem { tp: &type_float, permissions: READ_FLAG }];
    match st.call_sig(&[], &inputs).unwrap_err() {
        SigError::WrongType { found, wanted } => {
            assert_eq!(found.name, "int");
            assert_eq!(wanted.name, "float");
        }
        _ => panic!("Expected WrongType"),
    }
}

/*──────────────────────── missing argument ───────────────────*/

#[test]
fn sig_stack_missing_argument_error() {
    let type_int = make_int();

    let mut st  = SigStack::new();

    let inputs = [SigItem { tp: &type_int, permissions: READ_FLAG }];
    match st.call_sig(&[], &inputs).unwrap_err() {
        SigError::MissingArgument(item) => assert_eq!(item.tp.name, "int"),
        _ => panic!("Expected MissingArgument"),
    }
}

/*──────────────────────── permission clash ───────────────────*/

#[test]
fn sig_stack_permission_error() {
    let type_int = make_int();

    let mut st  = SigStack::new();

    // int variable, READ only
    let borrow_id = st.add_borrows(0);
    st.var_arena
        .push(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            borrow_id,
            permissions: READ_FLAG,
        });
    let var1 = st.var_arena.len() - 1;
    st.stack.push(var1);

    let inputs = [SigItem { tp: &type_int, permissions: WRITE_FLAG }];
    match st.call_sig(&[], &inputs).unwrap_err() {
        SigError::BasicSigError { clash, have } => {
            assert!(clash & WRITE_FLAG != 0);
            assert_eq!(have & WRITE_FLAG, 0);
        }
        _ => panic!("Expected BasicSigError"),
    }
}

/*──────────────────────── needs unique ───────────────────────*/

#[test]
fn sig_stack_needs_unique_error() {
    let type_int = make_int();

    let mut st  = SigStack::new();

    // borrow count = 1  (already shared-borrowed)
    let borrow_id = st.add_borrows(1);
    st.var_arena
        .push(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            borrow_id,
            permissions: READ_FLAG | UNIQUE_FLAG,
        });
    let var = st.var_arena.len() - 1;

    st.stack.push(var);
    st.stack.push(var);

    let inputs = [
        SigItem { tp: &type_int, permissions: UNIQUE_FLAG },
        SigItem { tp: &type_int, permissions: UNIQUE_FLAG },
    ];
    assert!(matches!(st.call_sig(&[], &inputs).unwrap_err(), SigError::NeedsUnique));
}

/*──────────────────────── already borrowed ───────────────────*/

#[test]
fn sig_stack_already_borrowed_error() {
    let type_int = make_int();

    let mut st  = SigStack::new();

    // borrow count = -1  (currently unique-borrowed)
    let borrow_id = st.add_borrows(-1);
    st.var_arena
        .push(CompVar {
            tp: &type_int,
            offset_from_start: 0,
            borrow_id,
            permissions: READ_FLAG | UNIQUE_FLAG,
        });
    let var = st.var_arena.len() - 1;
    st.stack.push(var);

    let inputs = [SigItem { tp: &type_int, permissions: READ_FLAG }];
    assert!(matches!(st.call_sig(&[], &inputs).unwrap_err(), SigError::AlreadyBorrowed));
}