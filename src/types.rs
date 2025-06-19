use core::fmt;

pub const READ_FLAG: u8   = 0x1;
pub const WRITE_FLAG: u8  = 0x2;
pub const UNIQUE_FLAG: u8 = 0x4;
pub const OUTPUT_FLAG: u8 = 0x8;

pub type RwT = u8;

pub enum SigError<'lex>{
    WrongType{found:&'lex Type<'lex>,wanted:&'lex Type<'lex>},//puting invalids here is UB
    NeedsUnique,
	AlreadyBorrowed,
	BasicSigError(RwT),//first 4bits which type of error later 4bits whether sig has that field on
}
impl fmt::Display for SigError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigError::WrongType { found, wanted } => {
                writeln!(f, "  - expected {}, but got {}", wanted.name, found.name)
            }
            SigError::NeedsUnique => write!(
                f,
                "Cannot borrow mutably: value is already borrowed (requires unique access)"
            ),
            SigError::AlreadyBorrowed => write!(
                f,
                "Cannot borrow mutably: value is currently borrowed as unique"
            ),
            SigError::BasicSigError(flags) => {
                let have = flags & 0x0F;
                let want = (flags & 0xF0) >> 4;

                writeln!(f, "Signature mismatch:")?;

                if (want & READ_FLAG != 0) && (have & READ_FLAG == 0) {
                    writeln!(f, "  - expected read access, but it's missing")?;
                }
                if (want & WRITE_FLAG != 0) && (have & WRITE_FLAG == 0) {
                    writeln!(f, "  - expected write access, but it's missing")?;
                }
                if (want & UNIQUE_FLAG != 0) && (have & UNIQUE_FLAG == 0) {
                    writeln!(f, "  - expected unique access, but it's missing")?;
                }
                if (want & OUTPUT_FLAG != 0) != (have & OUTPUT_FLAG != 0) {
                    let expected = if want & OUTPUT_FLAG != 0 { "output" } else { "input" };
                    let actual   = if have & OUTPUT_FLAG != 0 { "output" } else { "input" };
                    writeln!(f, "  - expected {}, but got {}", expected, actual)?;
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

#[derive(Debug)]
pub struct Type<'lex>{
    pub size:usize,
    pub name:&'lex str,
    pub inner:TypeInner<'lex>,
}

#[derive(Debug)]
pub enum TypeInner<'lex>{
    Basic,
    Alias(&'lex Type<'lex>),
    Array(&'lex Type<'lex>),
    Tuple(&'lex [Type<'lex>]),
}

#[derive(Debug)]
pub struct SigItem<'lex>{
    pub tp: &'lex Type<'lex>,
    pub permissions: RwT, 
}
impl fmt::Display for SigItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [", self.tp.name)?;

        let mut first = true;
        if self.permissions & READ_FLAG != 0 {
            if !first { write!(f, ", ")?; }
            write!(f, "read")?;
            first = false;
        }
        if self.permissions & WRITE_FLAG != 0 {
            if !first { write!(f, ", ")?; }
            write!(f, "write")?;
            first = false;
        }
        if self.permissions & UNIQUE_FLAG != 0 {
            if !first { write!(f, ", ")?; }
            write!(f, "unique")?;
            first = false;
        }
        if self.permissions & OUTPUT_FLAG != 0 {
            if !first { write!(f, ", ")?; }
            write!(f, "output")?;
        }

        write!(f, "]")
    }
}


#[derive(Debug)]
pub struct BoxVar<'lex> {
    pub tp: &'lex Type<'lex>,
    pub offset_from_start: i32, // first local is 0
    pub num_borrowed: i32,      // unique borrow is -1
    pub permissions: RwT,
}


fn check_subset(box_perm: RwT, sig: RwT) -> Result<(),SigError<'static>> {
    let mut ans = 0;

    if (sig & UNIQUE_FLAG != 0) && (box_perm & UNIQUE_FLAG == 0) {
        ans |= UNIQUE_FLAG;
        ans |= (sig & UNIQUE_FLAG)>>4;
    }
    if (sig & WRITE_FLAG != 0) && (box_perm & WRITE_FLAG == 0) {
        ans |= WRITE_FLAG;
        ans |= (sig & WRITE_FLAG)>>4;
    }
    if (sig & READ_FLAG != 0) && (box_perm & READ_FLAG == 0) {
        ans |= READ_FLAG;
        ans |= (sig & READ_FLAG)>>4;
    }
    if (sig & OUTPUT_FLAG) != (box_perm & OUTPUT_FLAG) {
        ans |= OUTPUT_FLAG;
        ans |= (sig & OUTPUT_FLAG)>>4;
    }
    
    if ans!=0 {
    	Err(SigError::BasicSigError(ans))
    }else{
    	Ok(())
    }
}

pub fn use_box_as<'lex>(box_var: &mut BoxVar<'lex>, sig: SigItem<'lex>) -> Result<(),SigError<'lex>> {
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

pub fn free_box_use(box_var: &mut BoxVar, sig: RwT) {
    if sig & UNIQUE_FLAG != 0 {
        box_var.num_borrowed = 0;
    } else {
        box_var.num_borrowed -= 1;
    }
}

// pub fn check_sig<'lex>(box_var: &mut StackRef<'a, Type>, sig: SigItem<'lex>) -> Result<(),SigError<'lex>>{

// }
