use crate::Code;
use crate::PalHash;
use crate::ir::Word;
use crate::stack::StackVec;
use crate::types::Type;
use crate::types::TypeInner;
use crate::types::TypeP;
use core::fmt;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Write;
use core::hash::Hash;
use core::hash::Hasher;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ops::Index;
use core::ops::IndexMut;
use core::slice;

pub struct Lex<'lex> {
    pub code_mem: StackAllocator<'lex, Code>,
    pub data_mem: StackAlloc<'lex>,

    pub comp_data_mem: StackAlloc<'lex>,
    pub types_mem: StackAllocator<'lex, Type<'lex>>,
    pub type_map: PalHash<&'lex TypeInner<'lex>, TypeP<'lex>>,

    pub words: PalHash<&'lex str, Word<'lex>>,
}

// ───────────── STACK ALLOC (untyped, bytes) ────────────────────────────
pub struct StackAlloc<'a>(StackVec<'a, u8>);

#[derive(Debug, Clone, Copy)]
pub struct StackAllocCheckPoint(usize); // logical length at CP

impl<'lex> StackAlloc<'lex> {
    #[inline]
    pub fn from_slice(raw: &'lex mut [MaybeUninit<u8>]) -> Self {
        Self(StackVec::from_slice(raw))
    }

    #[inline(always)]
    pub fn alloc<T>(&mut self) -> Option<&'lex mut MaybeUninit<T>> {
        let curr_len = self.0.len();
        let curr_ptr = unsafe { self.0.base.add(curr_len) };

        /* built-in helper: bytes to add so `curr_ptr` satisfies `align_of::<T>()` */
        let pad = curr_ptr.align_offset(align_of::<T>());
        debug_assert!(pad != usize::MAX, "impossible alignment failure");

        let total = pad + size_of::<T>(); // pad + payload
        unsafe {
            self.0.alloc(total)?; // bump StackVec ↑
            let slot = curr_ptr.add(pad) as *mut MaybeUninit<T>;
            Some(&mut *slot)
        }
    }

    #[inline]
    pub fn check_point(&self) -> StackAllocCheckPoint {
        StackAllocCheckPoint(self.0.len())
    }

    /// # Safety
    /// No references into the region above the checkpoint may still be live.
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocCheckPoint) {
        let to_free = self.0.len() - cp.0;
        // Everything here is plain bytes, so dropping isn’t required.
        self.0.free(to_free).expect("checkpoint math is wrong");
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ───────────── WRITER (printf-style, returns &str) ─────────────────────
pub struct StackWriter<'me, 'lex> {
    alloc: &'me mut StackAlloc<'lex>,
    start: usize,
}

impl Write for StackWriter<'_, '_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.alloc.0.push_slice(s.as_bytes()).ok_or(fmt::Error)
    }
}

impl<'me, 'lex> StackWriter<'me, 'lex> {
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
            let body = core::slice::from_raw_parts_mut(start, len);
            core::str::from_utf8_unchecked_mut(body)
        }
    }
}

// ───────────── STACK ALLOC (typed) ─────────────────────────────────────
pub struct StackAllocator<'a, T>(StackVec<'a, T>);

#[derive(Debug, Clone, Copy)]
pub struct StackAllocatorCheckPoint(usize); // length in elements

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

    #[inline]
    pub fn check_point(&self) -> StackAllocatorCheckPoint {
        StackAllocatorCheckPoint(self.0.len())
    }

    #[inline]
    pub fn try_index_checkpoint(&self, cp: StackAllocatorCheckPoint) -> Option<&'a [T]> {
        let live = self.0.len() - cp.0;
        let addr = self.0.peek_many(live)?.as_ptr().addr();
        let p = self.0.peek_raw()?.with_addr(addr);
        unsafe { Some(slice::from_raw_parts(p, live)) }
    }

    #[inline]
    pub fn index_checkpoint(&self, cp: StackAllocatorCheckPoint) -> &'a [T] {
        self.try_index_checkpoint(cp)
            .expect("checkpoint math is wrong")
    }

    /// # Safety
    /// No live references into the abandoned tail may survive.
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self, cp: StackAllocatorCheckPoint) {
        let live = self.0.len() - cp.0;
        self.0.flush(live).expect("checkpoint math is wrong"); // drop each value
    }

    /// # Safety
    /// this internal stack lets you break all of the allocators assumbtions
    /// this function should only be used while viewing the code for the allocator itself
    #[inline(always)]
    pub unsafe fn get_inner(&mut self) -> &mut StackVec<'a, T> {
        &mut self.0
    }

    #[inline(always)]
    pub fn with_addr(&self, addr: usize) -> *mut T {
        self.0.base.with_addr(addr)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> Index<usize> for StackAllocator<'_, T> {
    type Output = T;
    #[inline]
    fn index(&self, i: usize) -> &T {
        &self.0[i]
    }
}
impl<T> IndexMut<usize> for StackAllocator<'_, T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.0[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::make_storage;
    use core::mem::{MaybeUninit, align_of};

    /// Helper: turn the reference we get back into an integer address.
    #[inline]
    fn addr_of<T>(slot: &mut MaybeUninit<T>) -> usize {
        slot as *mut _ as usize
    }

    /// A 1-byte payload that *demands* 32-byte alignment.
    /// (Rust will still round `size_of::<OverAligned>()` up to 32,
    /// so the object is smaller *logically* than its alignment requirement.)
    #[repr(align(32))]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct OverAligned(u8);

    struct Zst;

    #[test]
    fn stack_alloc_aligment() {
        // 1 KiB is plenty for the test; adjust if your arena requires more.
        let mut backing: [_; 1024] = make_storage();
        // Safety: we hand the arena exclusive access to `backing`.
        let mut arena = StackAlloc::from_slice(&mut backing);

        /* ── plain u16 (align = 2) ─────────────────────────────────── */
        let s1 = arena.alloc::<u16>().expect("u16 should fit");
        let a1 = addr_of(s1);
        assert_eq!(a1 % align_of::<u16>(), 0, "u16 not aligned");

        /* ── overalligned tiny struct (align = 32 > payload) ───────── */
        let s2 = arena.alloc::<OverAligned>().expect("OverAligned");
        let a2 = addr_of(s2);
        assert_eq!(a2 % align_of::<OverAligned>(), 0, "OverAligned mis-aligned");

        *s2 = MaybeUninit::new(OverAligned(2));
        unsafe { assert_eq!(s2.assume_init(), OverAligned(2)) }

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

//----------------- DELAYED REF -------------------

#[derive(Debug, Clone, Copy, Eq)]
pub struct DelayedRef<'a, T> {
    inner: *const T,
    _ph: PhantomData<&'a T>,
}

impl<T> Deref for DelayedRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner }
    }
}

impl<'a, T> From<&'a T> for DelayedRef<'a, T> {
    fn from(inner: &'a T) -> Self {
        Self {
            inner,
            _ph: PhantomData,
        }
    }
}
impl<'a, T> From<&'a mut T> for DelayedRef<'a, T> {
    fn from(inner: &'a mut T) -> Self {
        Self {
            inner,
            _ph: PhantomData,
        }
    }
}

impl<T> PartialEq for DelayedRef<'_, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &DelayedRef<'_, T>) -> bool {
        let a: &T = self;
        let b: &T = other;
        a == b
    }
}

impl<T> Hash for DelayedRef<'_, T>
where
    T: Hash,
{
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        let a: &T = self;
        a.hash(hasher)
    }
}

impl<T> DelayedRef<'_, T> {
    ///# Safety
    /// no safe code is allowed to deref it untill a proper offset_from is called
    pub unsafe fn new_offset(base: *const u8, ptr: &T) -> Self {
        let offset = unsafe { (ptr as *const T as *const u8).offset_from(base) };
        Self {
            inner: offset as *const T,
            _ph: PhantomData,
        }
    }
    ///# Safety
    ///base must be the correct base address where the memory is allocated
    pub unsafe fn offset_from(&mut self, base: *const u8) {
        unsafe {
            self.inner = base.add(self.inner as usize) as *const T;
        }
    }
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct DelayedSlice<'a, T> {
    ptr: *const T,
    len: usize,
    _ph: PhantomData<&'a [T]>,
}

impl<T> PartialEq for DelayedSlice<'_, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &DelayedSlice<'_, T>) -> bool {
        let a: &[T] = self;
        let b: &[T] = other;
        a == b
    }
}

impl<T> Hash for DelayedSlice<'_, T>
where
    T: Hash,
{
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        let a: &[T] = self;
        a.hash(hasher)
    }
}

impl<T> Deref for DelayedSlice<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> Index<usize> for DelayedSlice<'_, T> {
    type Output = T;
    fn index(&self, id: usize) -> &T {
        let s: &[T] = self;
        &s[id]
    }
}

impl<'a, T> From<&'a [T]> for DelayedSlice<'a, T> {
    fn from(slice: &'a [T]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _ph: PhantomData,
        }
    }
}

impl<'a, T> From<&'a mut [T]> for DelayedSlice<'a, T> {
    fn from(slice: &'a mut [T]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _ph: PhantomData,
        }
    }
}

impl<T> DelayedSlice<'_, T> {
    ///# Safety
    /// no safe code is allowed to deref it untill a proper offset_from is called
    pub unsafe fn new_offset(base: *const u8, slice: &[T]) -> Self {
        let s = unsafe { (slice.as_ptr() as *const u8).offset_from(base) };
        Self {
            ptr: s as *const T,
            len: slice.len(),
            _ph: PhantomData,
        }
    }
    ///# Safety
    ///base must be the correct base address where the memory is allocated
    pub unsafe fn offset_from(&mut self, base: *const u8) {
        unsafe {
            self.ptr = base.add(self.ptr.addr()) as *const T;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DelayedStr<'a>(pub DelayedSlice<'a, u8>);

impl Deref for DelayedStr<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.0) }
    }
}

impl<'a> From<&'a str> for DelayedStr<'a> {
    fn from(s: &'a str) -> Self {
        Self(s.as_bytes().into())
    }
}

impl DelayedStr<'_> {
    ///# Safety
    /// no safe code is allowed to deref it untill a proper offset_from is called
    pub unsafe fn new_offset(base: *const u8, s: &str) -> Self {
        unsafe { Self(DelayedSlice::new_offset(base, s.as_bytes())) }
    }
    ///# Safety
    ///base must be the correct base address where the memory is allocated
    pub unsafe fn offset_from(&mut self, base: *const u8) {
        unsafe { self.0.offset_from(base) }
    }
}

impl Display for DelayedStr<'_> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        fmt.write_str(self)
    }
}

#[test]
fn test_string_delayed() {
    let base = "---hello world";
    let s = base.trim_start_matches('-');

    let mut delayed = unsafe { DelayedStr::new_offset(base.as_ptr(), s) };
    unsafe {
        delayed.offset_from(base.as_ptr());
    }

    assert_eq!(s, &*delayed)
}
