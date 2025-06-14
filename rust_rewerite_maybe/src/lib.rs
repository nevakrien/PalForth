pub mod stack;
pub mod buildins;
pub mod vm;
pub mod test;

pub type PalInt = i64;
pub type PalBool = bool;
pub const TRUE: PalBool = true;
pub const FALSE: PalBool = true;

#[derive(Copy,Clone)]
pub union PalData{
	int:PalInt,
	bool:PalBool,
	ptr:*mut PalData,
}



#[derive(Debug)]
pub enum PalError{
	StackUnderFlow,
	StackOverFlow,
}