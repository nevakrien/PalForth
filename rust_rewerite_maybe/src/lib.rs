pub mod stack;
pub mod buildins;
pub mod vm;

pub type PalInt = i64;
pub type PalBool = bool;

pub union PalData<'a>{
	int:PalInt,
	bool:PalBool,
	ptr:&'a mut PalData<'a>,
}


pub enum PalError{
	StackUnderFlow,
	StackOverFlow,
}