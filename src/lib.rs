#![cfg_attr(not(feature = "std"), no_std)]

use crate::vm::Code;
use hashbrown::HashMap;

pub mod stack;

pub mod buildins;
pub mod ir;
pub mod input;
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
pub enum PalError {
    StackUnderFlow,
    StackOverFlow,
}
