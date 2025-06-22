use crate::stack::StackVec;
use core::fmt;
use core::fmt::Write;
use core::ops::IndexMut;
use core::ops::Index;
use crate::types::PalTypeId;
use crate::PalHash;
use crate::types::TypeInner;
use crate::Code;
use crate::types::Type;
use core::{mem::MaybeUninit};
use crate::stack::StackRef;



pub struct Lex<'lex>{
	pub code_mem:StackAllocator<'lex,Code>,
	pub data_mem:StackAlloc<'lex>,
	
	pub comp_data_mem:StackAlloc<'lex>,
	pub types_mem:StackAllocator<'lex,Type<'lex>>,
	pub type_map:PalHash<&'lex TypeInner<'lex>,PalTypeId>,
}

// //========= STACK ALLOC=============

// pub struct StackAlloc<'a>(StackRef<'a, u8>);

// #[derive(Debug,Clone,Copy)]
// pub struct StackAllocCheckPoint(*mut u8);

// impl<'lex> StackAlloc<'lex> {
// 	#[inline]
// 	pub fn new(s:StackRef<'lex, u8>)->Self{
// 		StackAlloc(s)
// 	}

// 	#[inline(always)]//we dont want 20 of these
// 	pub fn alloc<T>(&mut self) -> Option<&'lex mut core::mem::MaybeUninit<T>> {
// 		//this may seem like its unsound but StackRef holds the memory UNIQUELY
// 		//for the duration of StackAlloc. so the only thing that can write to that memory is us

// 	    /* ── ZST handling ──────────────────────────────────────────── */
// 	    if size_of::<T>() == 0 {
// 	        // Return a pointer *inside* the arena (head), and leave the
// 	        // bump pointer untouched so the test’s               a3 == a2
// 	        // invariant holds.
// 	        return Some(unsafe { &mut *(self.0.head as *mut MaybeUninit<T>) });
// 	    }

// 	    /* ── constants for non-ZSTs ───────────────────────────────── */
// 	    let size        = size_of::<T>();
// 	    let align_mask  = align_of::<T>() - 1;          // power-of-two − 1
// 	    let curr        = self.0.head as usize;         // byte above the free space
// 	    let start       = (curr - size) & !align_mask;  // round *down* to align
// 	    let total       = curr - start;                 // pad + size

// 	    let ptr = self.0.head.with_addr(start)  as *mut MaybeUninit<T>;

// 	    /* ── delegate capacity check to the stack ─────────────────── */
// 	    unsafe {
// 	        self.0.alloc(total)?;                       // None ⇒ OOM
// 	        Some(&mut*ptr)
// 	    }
// 	}


// 	#[inline]
// 	pub fn check_point(&self)->StackAllocCheckPoint{
// 		StackAllocCheckPoint(self.0.head)
// 	}

// 	/// # Safety
// 	/// nothing points to memory we are currently freeing
// 	#[inline]
// 	pub unsafe fn goto_checkpoint(&mut self,check_point:StackAllocCheckPoint){
// 		self.0.head=check_point.0;
// 	}

// 	#[inline]
// 	pub fn len(&self) -> usize{
// 		self.0.len()
// 	}

// 	#[inline]
// 	pub fn is_empty(&self) -> bool{
// 		self.0.is_empty()
// 	}
// }

// //========= WRITING TO STACK ==============
// //TODO make a stack allocator that grows up so we dont need to invert a string here

// use core::fmt::{self, Write};
// use core::ptr::write;

// pub struct StackWriter<'me,'lex> {
//     alloc: &'me mut StackAlloc<'lex>,
//     start: *mut u8,
// }

// impl Write for StackWriter<'_,'_> {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         for &b in s.as_bytes() {
//             // Allocate 1 byte
//             let slot = self.alloc.alloc::<u8>().ok_or(fmt::Error)?;
//             unsafe { write(slot.as_mut_ptr(), b); }
//         }
//         Ok(())
//     }
// }

// impl<'me,'lex> StackWriter<'me,'lex> {
//     pub fn new(alloc: &'me mut StackAlloc<'lex>) -> Self {
//         let start = alloc.0.head;
//         StackWriter {
//             alloc,
//             start,
//         }
//     }

//     pub fn finish(self) -> &'lex mut str {unsafe{
//     	let len = self.start.offset_from(self.alloc.0.head) as usize;
//     	let slice = core::slice::from_raw_parts_mut(self.alloc.0.head, len);
//     	slice.reverse();
//         core::str::from_utf8_unchecked_mut(slice)
//     }}
// }

// //========= TYPED STACK ALLOC =============

// /// An arena-style bump allocator that grows **downward** and hands out
// /// `&'arena mut MaybeUninit<T>` for a *single, concrete* `T`.
// ///
// /// Compared with the untyped `StackAlloc<'a, u8>`:
// /// * no alignment math – the backing slice is `[MaybeUninit<T>]`
// /// * “size” is counted in *elements*, not bytes
// /// * still handles ZSTs without moving the head
// pub struct StackAllocator<'a, T>(StackRef<'a, T>);

// #[derive(Debug, Clone, Copy)]
// pub struct StackAllocatorCheckPoint<T>(*mut T);

// impl<T> Drop for StackAllocator<'_, T>{

// fn drop(&mut self) {
// 	while self.0.pop().is_some(){

// 	}
// }
// }

// impl<'a, T> StackAllocator<'a, T> {
//     #[inline]
//     pub fn new(storage: &'a mut [MaybeUninit<T>]) -> Self {
//         Self(StackRef::from_slice(storage))
//     }

//     #[inline]
//     pub fn save(&mut self,elem:T)->Result<&'a mut T,T>{
//     	if size_of::<T>() == 0 {
//             return Ok(unsafe {&mut* core::ptr::dangling_mut()});
//         }

//     	unsafe{
//     		match self.0.alloc(1){
//     			None=>Err(elem),
//     			Some(_)=>{
//     				let mem = self.0.spot_raw(0).unwrap_unchecked();
//     				mem.write(elem);
//     				Ok(&mut*mem)
//     			}
//     		}
//     	}
//     }

//     /// A snapshot of the current bump pointer.
//     #[inline]
//     pub fn check_point(&self) -> StackAllocatorCheckPoint<T> {
//         StackAllocatorCheckPoint(self.0.head)
//     }

//     /// # Safety
//     /// Caller must guarantee that *nothing* still points inside the region that
//     /// is being rolled back (identical to the byte allocator’s contract).
//     #[inline]
//     pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocatorCheckPoint<T>) { unsafe {
//     	while self.0.head!=cp.0{
//     		let _ = self.0.pop().unwrap_unchecked();
//     	}
//     }}

//     #[inline]
// 	pub fn len(&self) -> usize{
// 		self.0.len()
// 	}

// 	#[inline]
// 	pub fn is_empty(&self) -> bool{
// 		self.0.is_empty()
// 	}
// }

// impl<T> Index<usize> for StackAllocator<'_, T>{

// type Output = T;
// fn index(&self, idx: usize) -> &T { &self.0[idx] }
// }

// impl<T> IndexMut<usize> for StackAllocator<'_, T>{

// fn index_mut(&mut self, idx: usize) -> &mut T { &mut self.0[idx] }
// }





// ───────────── STACK ALLOC (untyped, bytes) ────────────────────────────
pub struct StackAlloc<'a>(StackVec<'a, u8>);

#[derive(Debug, Clone, Copy)]
pub struct StackAllocCheckPoint(usize);        // logical length at CP

impl<'lex> StackAlloc<'lex> {
    #[inline]
    pub fn from_slice(raw: &'lex mut [MaybeUninit<u8>]) -> Self {
        Self(StackVec::from_slice(raw))
    }

    #[inline(always)]
	pub fn alloc<T>(&mut self) -> Option<&'lex mut MaybeUninit<T>> {
	    let curr_len  = self.0.len();
	    let curr_ptr  = unsafe {self.0.base.add(curr_len)};

	    /* built-in helper: bytes to add so `curr_ptr` satisfies `align_of::<T>()` */
	    let pad = curr_ptr.align_offset(align_of::<T>());
	    debug_assert!(pad != usize::MAX, "impossible alignment failure");

	    let total = pad + size_of::<T>();                       // pad + payload
	    unsafe {
	        self.0.alloc(total)?;                               // bump StackVec ↑
	        let slot  = curr_ptr.add(pad) as *mut MaybeUninit<T>;
	        Some(&mut *slot)
	    }
	}


    #[inline] pub fn check_point(&self) -> StackAllocCheckPoint {
        StackAllocCheckPoint(self.0.len())
    }

    /// # Safety
    /// No references into the region above the checkpoint may still be live.
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocCheckPoint) {
        let to_free = self.0.len() - cp.0;
        // Everything here is plain bytes, so dropping isn’t required.
        self.0.free(to_free).unwrap();
    }

    #[inline] pub fn len(&self) -> usize       { self.0.len() }
    #[inline] pub fn is_empty(&self) -> bool   { self.0.is_empty() }
}

// ───────────── WRITER (printf-style, returns &str) ─────────────────────
pub struct StackWriter<'me,'lex> {
    alloc: &'me mut StackAlloc<'lex>,
    start: usize,
}

impl Write for StackWriter<'_,'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.alloc.0.push_slice(s.as_bytes()).ok_or(fmt::Error)
    }
}

impl<'me,'lex> StackWriter<'me,'lex> {
    #[inline]
    pub fn new(alloc: &'me mut StackAlloc<'lex>) -> Self {
        let start = alloc.0.len();
        Self { alloc, start }
    }

    #[inline]
    pub fn finish(self) -> &'lex mut str {
        unsafe {
        	let start = self.alloc.0.base.add(self.start);
        	let len = self.alloc.0.len - self.start;
            let body  = core::slice::from_raw_parts_mut(start,len);
            core::str::from_utf8_unchecked_mut(body)
        }
    }
}

// ───────────── STACK ALLOC (typed) ─────────────────────────────────────
pub struct StackAllocator<'a, T>(StackVec<'a, T>);

#[derive(Debug, Clone, Copy)]
pub struct StackAllocatorCheckPoint(usize);    // length in elements

impl<'a, T> StackAllocator<'a, T> {
    #[inline]
    pub fn new(buf: &'a mut [MaybeUninit<T>]) -> Self {
        Self(StackVec::from_slice(buf))
    }

    #[inline]
    pub fn save(&mut self, elem: T) -> Result<&'a mut T, T> {
        if size_of::<T>() == 0 {
            return Ok(unsafe { &mut *core::ptr::dangling_mut() });
        }

        unsafe {
            match self.0.alloc(1) {
                None => Err(elem),
                Some(_) => {
                    let slot = self.0.peek_raw().unwrap_unchecked();
                    slot.write(elem);
                    Ok(&mut *slot)
                }
            }
        }
    }

    #[inline] pub fn check_point(&self) -> StackAllocatorCheckPoint {
        StackAllocatorCheckPoint(self.0.len())
    }

    /// # Safety
    /// No live references into the abandoned tail may survive.
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocatorCheckPoint) {
        let live = self.0.len() - cp.0;
        self.0.flush(live).unwrap();   // drop each value
    }

    #[inline] pub fn len(&self) -> usize       { self.0.len() }
    #[inline] pub fn is_empty(&self) -> bool   { self.0.is_empty() }
}

impl<T> Index<usize> for StackAllocator<'_, T> {
    type Output = T;
    #[inline] fn index(&self, i: usize) -> &T       { &self.0[i] }
}
impl<T> IndexMut<usize> for StackAllocator<'_, T> {
    #[inline] fn index_mut(&mut self, i: usize) -> &mut T { &mut self.0[i] }
}



#[cfg(test)]
mod tests {
    use crate::stack::make_storage;
use super::*;
    use core::mem::{align_of, MaybeUninit};

    /// Helper: turn the reference we get back into an integer address.
    #[inline]
    fn addr_of<T>(slot: &mut MaybeUninit<T>) -> usize {
        slot as *mut _ as usize
    }

    /// A 1-byte payload that *demands* 32-byte alignment.
    /// (Rust will still round `size_of::<OverAligned>()` up to 32,
    /// so the object is smaller *logically* than its alignment requirement.)
    #[repr(align(32))]
	#[derive(Copy,Clone,Debug,PartialEq)]
	struct OverAligned(u8);

	struct Zst;


    #[test]
    fn stack_alloc_aligment() {
        // 1 KiB is plenty for the test; adjust if your arena requires more.
        let mut backing: [_; 1024] = make_storage();
        // Safety: we hand the arena exclusive access to `backing`.
        let mut arena =StackAlloc::from_slice(&mut backing);

        /* ── plain u16 (align = 2) ─────────────────────────────────── */
        let s1 = arena.alloc::<u16>().expect("u16 should fit");
        let a1 = addr_of(s1);
        assert_eq!(a1 % align_of::<u16>(), 0, "u16 not aligned");

        /* ── overalligned tiny struct (align = 32 > payload) ───────── */
        let s2 = arena.alloc::<OverAligned>().expect("OverAligned");
        let a2 = addr_of(s2);
        assert_eq!(a2 % align_of::<OverAligned>(), 0, "OverAligned mis-aligned");

        *s2=MaybeUninit::new(OverAligned(2));
        unsafe{
        	assert_eq!(s2.assume_init(),OverAligned(2))

        }

        /* ── zero-sized type (size = 0, align = 1) ─────────────────── */
        let s3 = arena.alloc::<Zst>().expect("ZST");
        *s3 = MaybeUninit::new(Zst);

        let a3 = addr_of(s3);
        assert_eq!(a3 % align_of::<()>(), 0);

        /* ── an array with odd size/alignment interplay ────────────── */
        let s4 = arena.alloc::<[u64; 3]>().expect("[u64;3]");
        let a4 = addr_of(s4);
        assert_eq!(a4 % align_of::<[u64; 3]>(), 0, "array mis-aligned");

        /* ── near-exhaustion check: fill what’s left in 8-byte chunks ─ */
        loop {
            match arena.alloc::<u64>() {
                Some(_) => continue,
                None => break, // expected out-of-memory
            }
        }
        assert!(arena.alloc::<u64>().is_none(), "OOM must remain OOM");
    }

    // #[test]
    // fn stack_alloc_aligment() {
    //     // 1 KiB is plenty for the test; adjust if your arena requires more.
    //     let mut backing: [_; 1024] = make_storage();
    //     // Safety: we hand the arena exclusive access to `backing`.
    //     let mut arena =StackAlloc::from_slice(&mut backing);

    //     /* ── plain u16 (align = 2) ─────────────────────────────────── */
    //     let s1 = arena.alloc::<u16>().expect("u16 should fit");
    //     let a1 = addr_of(s1);
    //     assert_eq!(a1 % align_of::<u16>(), 0, "u16 not aligned");

    //     /* ── overalligned tiny struct (align = 32 > payload) ───────── */
    //     let s2 = arena.alloc::<OverAligned>().expect("OverAligned");
    //     let a2 = addr_of(s2);
    //     assert_eq!(a2 % align_of::<OverAligned>(), 0, "OverAligned mis-aligned");

    //     *s2=MaybeUninit::new(OverAligned(2));
    //     unsafe{
    //     	assert_eq!(s2.assume_init(),OverAligned(2))

    //     }

    //     /* ── zero-sized type (size = 0, align = 1) ─────────────────── */
    //     let s3 = arena.alloc::<Zst>().expect("ZST");
    //     *s3 = MaybeUninit::new(Zst);

    //     let a3 = addr_of(s3);
    //     assert_eq!(a3 % align_of::<()>(), 0);
    //     // ZST must not consume space, so address should equal the last head.
    //     assert_eq!(a3, a2, "ZST should not move head");

    //     /* ── an array with odd size/alignment interplay ────────────── */
    //     let s4 = arena.alloc::<[u64; 3]>().expect("[u64;3]");
    //     let a4 = addr_of(s4);
    //     assert_eq!(a4 % align_of::<[u64; 3]>(), 0, "array mis-aligned");
    //     assert!(a4 < a2, "arena still grows downward");

    //     /* ── sanity: nothing overlapped and order is monotone ──────── */
    //     assert!(a1 > a2 && a2 >= a3 && a3 > a4);

    //     /* ── near-exhaustion check: fill what’s left in 8-byte chunks ─ */
    //     loop {
    //         match arena.alloc::<u64>() {
    //             Some(_) => continue,
    //             None => break, // expected out-of-memory
    //         }
    //     }
    //     assert!(arena.alloc::<u64>().is_none(), "OOM must remain OOM");
    // }

    #[test]
	fn stack_writer_write_and_finish() {
	    let mut backing: [_; 1024] = make_storage(); // 1 KiB arena
	    let mut arena = StackAlloc::from_slice(&mut backing);

	    let mut writer = StackWriter::new(&mut arena);
	    write!(writer, "hello").unwrap();
	    write!(writer, " world {}", 42).unwrap();

	    let result = writer.finish();
	    assert_eq!(result, "hello world 42");

	    // Make sure what we wrote is indeed valid and no extra allocations happened
	    let remaining_space = arena.len();
	    let used_bytes = 1024 - remaining_space;

	    assert!(
	        used_bytes >= result.len(),
	        "allocator should have used at least result length"
	    );
	}



	#[test]
	fn test_stack_allocator_basic() {
	    extern crate std;
	    use std::boxed::Box;

	    let mut storage = [const { MaybeUninit::<Box<i32>>::uninit() }; 8];
	    let mut alloc = StackAllocator::new(&mut storage);

	    let a = alloc.save(Box::new(10)).unwrap();
	    let b = alloc.save(Box::new(20)).unwrap();

	    assert_eq!(**a, 10);
	    assert_eq!(**b, 20);

	    let cp = alloc.check_point();
	    let c = alloc.save(Box::new(30)).unwrap();
	    assert_eq!(*c, Box::new(30));

	    unsafe {
	        alloc.goto_checkpoint(cp);
	    }

	    // allocation after rollback should overwrite 30
	    let d = alloc.save(Box::new(99)).unwrap();
	    assert_eq!(*d, Box::new(99));
	}
}
