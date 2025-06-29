use crate::lex::DelayedRef;
use crate::lex::DelayedStr;
use crate::lex::DelayedSlice;
use core::fmt::Write;
use crate::lex::StackWriter;
use core::cell::RefCell;
use crate::lex::StackAllocator;
use crate::stack::StackRef;
use crate::lex::Lex;
use core::fmt;

pub const READ_FLAG  : u8 = 0x1;
pub const WRITE_FLAG : u8 = 0x2;
pub const UNIQUE_FLAG: u8 = 0x4;
pub const OUTPUT_FLAG: u8 = 0x8;//whethere or not this stays on the output stack
pub const RAW_FLAG   : u8 = 0xA;//if set the value is passed on the data stack (this convention cant be easily automated)
pub const INDEX_FLAG : u8 = 0xB;//only relvent for outputs if set the return pointer may be ANY pointer Derived!!! from the input (which has lifetime implications) this convention cant be easily automated
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
            (false, true)  => AccessMode::Indexed,
            (true, false)  => AccessMode::Raw,
            (true, true)   => AccessMode::Invalid,
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
    WrongType { found: &'lex Type<'lex>, wanted: &'lex Type<'lex> },
    NeedsUnique,
    AlreadyBorrowed,
    BasicSigError { clash: RwT, have: RwT }, // cleaner and clearer
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
            SigError::AlreadyBorrowed => write!(
                f,
                "Cannot borrow: value is currently borrowed as unique"
            ),
            SigError::BasicSigError { clash, have } => {

                writeln!(f, "Signature mismatch:")?;

                if clash & READ_FLAG != 0{
                    writeln!(f, "  - expected read access, but it's missing")?;
                }
                if clash & WRITE_FLAG != 0 {
                    writeln!(f, "  - expected write access, but it's missing")?;
                }
                if clash & UNIQUE_FLAG != 0 {
                    writeln!(f, "  - expected unique access, but it's missing")?;
                }
                if clash & OUTPUT_FLAG != 0  {
                    let actual   = if have  & OUTPUT_FLAG != 0 { "output" } else { "input" };
                    let expected = if !have & OUTPUT_FLAG != 0 { "input" } else { "output" };
                    writeln!(f, "  - expected {}, but got {}", expected, actual)?;
                }

               if (clash & (RAW_FLAG | INDEX_FLAG)) != 0 {
                    let expected = AccessMode::from_bits(*clash);
                    let actual = AccessMode::from_bits(*have);

                    // Skip if AccessMode::Invalid – might be a programming error
                    if expected != actual {
                        writeln!(f, "  - expected {}, but got {}", expected.name(), actual.name())?;
                    }
                }


                Ok(())
            },
        }
    }
}

impl fmt::Debug for SigError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f,"SigError(\"{self}\")")
    }
}

/// these should be explicitly in types_mem
pub type TypeP<'lex> = DelayedRef<'lex,Type<'lex>>;

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]
pub struct Type<'lex>{
    pub inner:TypeInner<'lex>,
    pub size:i32,
    pub cells:i32,
    pub name:DelayedStr<'lex>,
}


#[derive(Debug,Clone,Copy,Eq,Hash,PartialEq)]
pub enum TypeInner<'lex>{
    Basic,
    Alias(TypeP<'lex>,&'lex str),
    Array(TypeP<'lex>,Option<i32>),
    Cluster(DelayedSlice<'lex,TypeP<'lex>>),
}

impl<'lex> TypeInner<'lex>{
    pub fn get_type_ref(&self,lex:&mut Lex<'lex>) -> TypeP<'lex>{
        if let Some(x) = lex.type_map.get(self) {
            return *x;
        }

        let (name,cells,size) = match self {
            TypeInner::Basic => unreachable!("missing basic type in the table"),
            TypeInner::Alias(parent,name)=>{
                (*name,parent.cells,parent.size)
            },
            TypeInner::Array(elem,num) => {
                let mut writer = StackWriter::new(&mut lex.comp_data_mem);
                match num{
                    None => {
                       write!(writer,"Array({})",elem.name).unwrap();
                       let (cells,size) =(2,2*size_of::<*const ()>());
                       (&*writer.finish(),cells as i32,size as i32)
                    },
                    Some(len) => {
                        write!(writer,"Array<{}>({})",len,elem.name).unwrap();
                        (&*writer.finish(),len*elem.cells,len*elem.size)
                    }
                }
                // 

            },
            TypeInner::Cluster(elems) => {
                let mut writer = StackWriter::new(&mut lex.comp_data_mem);
                todo!()
            }
        };
        let me : &'lex Type<'lex>= lex.types_mem.save(Type{
            inner:self.clone(),
            name: name.into(),
            cells,
            size,

        }).unwrap() as &Type;

        lex.type_map.insert(&me.inner,me.into()).unwrap();
        me.into()
    }
}

// pub fn get_tp<'lex>(lex:&mut Lex<'lex>,t:TypeInner<'lex>)-> &'lex Type<'lex>{
//     if let Some(x) = lex.type_map.get(&t){
//         return x;
//     }

//     let v = Type{inner:k,name:}

//     todo!()
// }

#[derive(Debug)]
pub struct SigItem<'lex>{
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
                    if !first { write!(f, ", ")?; }
                    write!(f, $label)?;
                    first = false;
                }
            }
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
pub struct CompVar<'lex> {
    pub tp: &'lex Type<'lex>,
    pub offset_from_start: i32, // first local is 0
    pub num_borrowed: i32,      // unique borrow is -1
    pub permissions: RwT,
}


fn check_subset(have: RwT, sig: RwT) -> Result<(),SigError<'static>> {
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
    
    if clash!=0 {
    	Err(SigError::BasicSigError{clash,have})
    }else{
    	Ok(())
    }
}

pub fn use_box_as<'lex>(box_var: &mut CompVar<'lex>, sig: &SigItem<'lex>) -> Result<(),SigError<'lex>> {
    if box_var.tp as *const _ !=sig.tp as *const _{
        return Err(SigError::WrongType { found:box_var.tp, wanted:sig.tp})
    }
    check_subset(box_var.permissions, sig.permissions)?;

    if box_var.num_borrowed == -1 {
        return Err(SigError::AlreadyBorrowed);
    }
    if (sig.permissions & UNIQUE_FLAG != 0) && box_var.num_borrowed != 0 {
        return Err(SigError::NeedsUnique);
    }

    if sig.permissions & UNIQUE_FLAG != 0 {
        box_var.num_borrowed = -1;
    } else {
        box_var.num_borrowed += 1;
    }

    Ok(())
}

pub fn free_box_use(box_var: &mut CompVar, sig: RwT) {
    if sig & UNIQUE_FLAG != 0 {
        box_var.num_borrowed = 0;
    } else {
        box_var.num_borrowed -= 1;
    }
}

// pub fn check_sig<'lex>(box_var: &mut StackRef<'a, Type>, sig: SigItem<'lex>) -> Result<(),SigError<'lex>>{

// }

/// # Safety
/// changing any of the underlying stacks is considered unsound
pub struct SigStack<'me, 'lex>{
    cells_locals:i32,
    arena:StackAllocator<'me,RefCell<CompVar<'lex>>>,
    pub stack:StackRef<'me,&'me RefCell<CompVar<'lex>>>,
}

impl<'me, 'lex> SigStack<'me, 'lex>{
    pub fn add_local(&mut self,tp:&'lex Type<'lex>)->&'me RefCell<CompVar<'lex>>{
        let var = CompVar{
            tp,
            permissions:READ_FLAG&WRITE_FLAG&UNIQUE_FLAG,
            num_borrowed:0,
            offset_from_start:self.cells_locals,
        };
        let ans = self.arena.save(var.into()).expect("underflow local stack");
        self.cells_locals+=tp.cells;
        ans
    }

    pub fn call_sig(&mut self,sig:&[SigItem<'lex>])->Result<(),SigError<'lex>>{
        for t in sig.iter().rev(){
            match self.stack.pop(){
                None=>todo!(),
                Some(b)=>use_box_as(&mut b.borrow_mut(),t)?,
            };
        }
        Ok(())
    }
}