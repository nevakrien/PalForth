#![cfg_attr(not(feature = "std"), no_std)]

use crate::types::SigError;
use core::ptr::NonNull;
use no_std_io::io::{self,Write};
use crate::vm::Code;
use hashbrown::HashMap;

// extern crate alloc;
// use alloc::boxed::Box;

pub mod stack;

pub mod buildins;
pub mod input;
pub mod ir;
pub mod lex;
pub mod types;
pub mod vm;

#[cfg(test)]
pub mod test;

type PalHash<K, V> = HashMap<K, V>; // use default hasher and allocator

pub type PalInt = i64;
pub type PalBool = bool;
pub const TRUE: PalBool = true;
pub const FALSE: PalBool = false;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub union PalData {
    int: PalInt,
    bool: PalBool,
    // ptr:*mut PalData,
    code: *const Code,
}

#[derive(Debug)]
pub enum PalError<'a> {
    // StackUnderFlow,
    // StackOverFlow,
    SigError(SigError<'a>),
    Io(io::Error),
    Missingword(&'a str)
}

impl<'a> From<SigError<'a>> for PalError<'a>{
fn from(s: SigError<'a>) -> Self { PalError::SigError(s) }
}

impl From<io::Error> for PalError<'_>{
fn from(s: io::Error) -> Self { PalError::Io(s) }
}

#[cfg(feature = "std")]
pub type DefualtLogger = StdOutLogger;

#[cfg(not(feature = "std"))]
pub type DefualtLogger = NulLogger;


#[cfg(feature = "std")]
pub struct StdOutLogger;

#[cfg(feature = "std")]
impl StdOutLogger{
	pub fn new_ref<'a>()->&'a mut Self{
		unsafe{&mut*NonNull::dangling().as_ptr()}
	}
}

#[cfg(feature = "std")]
impl Write for StdOutLogger {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::stdout().write(buf)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

pub struct NulLogger;
impl NulLogger{
	pub fn new_ref<'a>()->&'a mut Self{
		unsafe{&mut*NonNull::dangling().as_ptr()}
	}
}

impl Write for NulLogger{
#[inline]
fn write(&mut self, s: &[u8]) -> Result<usize, io::Error> {Ok(s.len())}
#[inline]
fn flush(&mut self) -> Result<(), io::Error> {Ok(())}
}