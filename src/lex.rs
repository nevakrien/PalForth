use crate::types::TypeId;
use crate::PalHash;
use crate::types::TypeInner;
use crate::Code;
use crate::types::Type;
use core::{mem::MaybeUninit};
use crate::stack::StackRef;



pub struct Lex<'lex>{
	pub code_mem:StackRef<'lex,Code>,
	pub data_mem:StackAlloc<'lex>,
	
	pub gen_mem:StackAlloc<'lex>,
	pub types_mem:StackRef<'lex,Type<'lex>>,
	pub type_map:PalHash<&'lex TypeInner,PalTypeId>,
}

//========= STACK ALLOC=============

pub struct StackAlloc<'a>(StackRef<'a, u8>);

#[derive(Debug,Clone,Copy)]
pub struct StackAllocCheckPoint(*mut u8);

impl<'lex> StackAlloc<'lex> {
	#[inline]
	pub fn new(s:StackRef<'lex, u8>)->Self{
		StackAlloc(s)
	}

	#[inline(always)]//we dont want 20 of these
	pub fn alloc<T>(&mut self) -> Option<&'lex mut core::mem::MaybeUninit<T>> {
		//this may seem like its unsound but StackRef holds the memory UNIQUELY
		//for the duration of StackAlloc. so the only thing that can write to that memory is us

	    /* ── ZST handling ──────────────────────────────────────────── */
	    if size_of::<T>() == 0 {
	        // Return a pointer *inside* the arena (head), and leave the
	        // bump pointer untouched so the test’s               a3 == a2
	        // invariant holds.
	        return Some(unsafe { &mut *(self.0.head as *mut MaybeUninit<T>) });
	    }

	    /* ── constants for non-ZSTs ───────────────────────────────── */
	    let size        = size_of::<T>();
	    let align_mask  = align_of::<T>() - 1;          // power-of-two − 1
	    let curr        = self.0.head as usize;         // byte above the free space
	    let start       = (curr - size) & !align_mask;  // round *down* to align
	    let total       = curr - start;                 // pad + size

	    /* ── delegate capacity check to the stack ─────────────────── */
	    unsafe {
	        self.0.alloc(total)?;                       // None ⇒ OOM
	        Some(&mut *(start as *mut MaybeUninit<T>))
	    }
	}



	#[inline]
	pub fn check_point(&self)->StackAllocCheckPoint{
		StackAllocCheckPoint(self.0.head)
	}

	/// # Safety
	/// nothing points to memory we are currently freeing
	#[inline]
	pub unsafe fn goto_checkpoint(&mut self,check_point:StackAllocCheckPoint){
		self.0.head=check_point.0;
	}
}

//========= TYPED STACK ALLOC =============

/// An arena-style bump allocator that grows **downward** and hands out
/// `&'arena mut MaybeUninit<T>` for a *single, concrete* `T`.
///
/// Compared with the untyped `StackAlloc<'a, u8>`:
/// * no alignment math – the backing slice is `[MaybeUninit<T>]`
/// * “size” is counted in *elements*, not bytes
/// * still handles ZSTs without moving the head
pub struct StackAllocator<'a, T>(StackRef<'a, T>);

#[derive(Debug, Clone, Copy)]
pub struct StackAllocatorCheckPoint<T>(*mut T);

impl<T> Drop for StackAllocator<'_, T>{

fn drop(&mut self) {
	while self.0.pop().is_some(){

	}
}
}

impl<'a, T> StackAllocator<'a, T> {
    #[inline]
    pub fn new(storage: &'a mut [MaybeUninit<T>]) -> Self {
        Self(StackRef::from_slice(storage))
    }

    pub fn save(&mut self,elem:T)->Result<&'a mut T,T>{
    	if size_of::<T>() == 0 {
            return Ok(unsafe {&mut* core::ptr::dangling_mut()});
        }

    	unsafe{
    		match self.0.alloc(1){
    			None=>Err(elem),
    			Some(_)=>{
    				let mem = self.0.spot_raw(0).unwrap_unchecked();
    				mem.write(elem);
    				Ok(&mut*mem)
    			}
    		}
    	}
    }

    /// A snapshot of the current bump pointer.
    #[inline]
    pub fn check_point(&self) -> StackAllocatorCheckPoint<T> {
        StackAllocatorCheckPoint(self.0.head)
    }

    /// # Safety
    /// Caller must guarantee that *nothing* still points inside the region that
    /// is being rolled back (identical to the byte allocator’s contract).
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocatorCheckPoint<T>) { unsafe {
    	while self.0.head!=cp.0{
    		let _ = self.0.pop().unwrap_unchecked();
    	}
    }}
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
        let mut arena =StackAlloc::new(StackRef::from_slice(&mut backing));

        /* ── plain u16 (align = 2) ─────────────────────────────────── */
        let s1 = arena.alloc::<u16>().expect("u16 should fit");
        let a1 = addr_of(s1);
        assert_eq!(a1 % align_of::<u16>(), 0, "u16 not aligned");

        /* ── overalligned tiny struct (align = 32 > payload) ───────── */
        let s2 = arena.alloc::<OverAligned>().expect("OverAligned");
        let a2 = addr_of(s2);
        assert_eq!(a2 % align_of::<OverAligned>(), 0, "OverAligned mis-aligned");
        assert!(a2 < a1, "arena must grow downward");

        *s2=MaybeUninit::new(OverAligned(2));
        unsafe{
        	assert_eq!(s2.assume_init(),OverAligned(2))

        }

        /* ── zero-sized type (size = 0, align = 1) ─────────────────── */
        let s3 = arena.alloc::<Zst>().expect("ZST");
        *s3 = MaybeUninit::new(Zst);

        let a3 = addr_of(s3);
        assert_eq!(a3 % align_of::<()>(), 0);
        // ZST must not consume space, so address should equal the last head.
        assert_eq!(a3, a2, "ZST should not move head");

        /* ── an array with odd size/alignment interplay ────────────── */
        let s4 = arena.alloc::<[u64; 3]>().expect("[u64;3]");
        let a4 = addr_of(s4);
        assert_eq!(a4 % align_of::<[u64; 3]>(), 0, "array mis-aligned");
        assert!(a4 < a2, "arena still grows downward");

        /* ── sanity: nothing overlapped and order is monotone ──────── */
        assert!(a1 > a2 && a2 >= a3 && a3 > a4);

        /* ── near-exhaustion check: fill what’s left in 8-byte chunks ─ */
        loop {
            match arena.alloc::<u64>() {
                Some(_) => continue,
                None => break, // expected out-of-memory
            }
        }
        assert!(arena.alloc::<u64>().is_none(), "OOM must remain OOM");
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
