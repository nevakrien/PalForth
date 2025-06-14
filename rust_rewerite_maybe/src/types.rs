pub const READ_FLAG: u8   = 0x1;
pub const WRITE_FLAG: u8  = 0x2;
pub const UNIQUE_FLAG: u8 = 0x4;
pub const OUTPUT_FLAG: u8 = 0x8;

pub enum TypeError{
	AlreadyBorrowed,
	BadSig(u8),//first 4bits which type of error later 4bits whether sig has that field on
}

pub type RwT = u8;

#[derive(Debug)]
pub struct BoxVar {
    pub r#type: *mut core::ffi::c_void,
    pub offset_from_start: i32, // first local is 0
    pub num_borrowed: i32,      // unique borrow is -1
    pub permissions: RwT,
}


fn check_subset(box_perm: RwT, sig: RwT) -> Result<(),TypeError> {
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
    	Err(TypeError::BadSig(ans))
    }else{
    	Ok(())
    }
}

pub fn use_box_as(box_var: &mut BoxVar, sig: RwT) -> Result<(),TypeError> {
    check_subset(box_var.permissions, sig)?;

    if box_var.num_borrowed == -1 {
        return Err(TypeError::AlreadyBorrowed);
    }
    if (sig & UNIQUE_FLAG != 0) && box_var.num_borrowed != 0 {
        return Err(TypeError::AlreadyBorrowed);
    }

    if sig & UNIQUE_FLAG != 0 {
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
