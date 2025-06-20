//this module has a lot of raw pointers betterbe explicit
#![allow(clippy::needless_lifetimes)]


use core::slice::from_raw_parts_mut;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;
use core::slice;



pub fn make_storage<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY: MaybeUninit is safe to use uninitialized.
    unsafe { MaybeUninit::uninit().assume_init() }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct StackCheckPoint<T>(*mut T);

/*──────────────────── stack type ───────────────────────*/

/// A **down-growing** fixed-capacity LIFO stack.
///
/// Layout (high address at the top):
///
/// ```text
///                    above ┐
///                          ▼
///   [ x x x x ...free... ][ ...dead space ...]
///     ▲     ▲            
///     │     └─ head      
///     └──────── end (low addr)
/// ```
pub struct StackRef<'mem, T> {
    //the fact rust is missing const makes this make less effishent code than i would like...
    pub(crate) above: *mut T,   // one-past the *highest* live element
    pub(crate) head:  *mut T,   // next pop / current top (lowest live element)
    pub(crate) end:   *mut T,   // lowest address in the backing buffer
    pub(crate) _ph:   PhantomData<&'mem mut [MaybeUninit<T>]>,
}

unsafe impl<'m, T: Send> Send for StackRef<'m, T> {}
unsafe impl<'m, T: Sync> Sync for StackRef<'m, T> {}

/*──────────────────── constructors ────────────────────*/
impl<'mem, T> StackRef<'mem, T> {
    /// Empty stack over an *uninitialised* slice.
    #[inline]
    pub const fn from_slice(buf: &'mem mut [MaybeUninit<T>]) -> Self {
        let end   = buf.as_mut_ptr() as *mut T;       // low addr
        let above = unsafe { end.add(buf.len()) };    // one-past high addr
        Self { above, head: above, end, _ph: PhantomData }
    }


    /// Empty stack over an *uninitialised* slice.
    #[inline]
    pub const fn from_slice_raw(buf: *mut [MaybeUninit<T>]) -> Self {
        let end   = buf as *mut T;       // low addr
        let above = unsafe { end.add(buf.len()) };    // one-past high addr
        Self { above, head: above, end, _ph: PhantomData }
    }

    /// Stack that is **full** (all elements initialised).
    #[inline]
    pub const fn new_full(buf: &'mem mut [T]) -> Self
    where
        T: Copy,
    {
        let end   = buf.as_mut_ptr();                 // low addr
        let above = unsafe { end.add(buf.len()) };    // one-past high addr
        Self { above, head: end, end, _ph: PhantomData }
    }

    /// Convert back to the original uninitialised slice.
    #[inline]
    pub const fn to_slice(self) -> &'mem mut [MaybeUninit<T>] {
        unsafe {
            let len = self.above.offset_from(self.end) as usize;
            from_raw_parts_mut(self.end as *mut _, len)
        }
    }

/*──────────────────── invariants ───────────────────────*/
    /// Number of live elements.
    #[inline]
    pub fn write_index(&self) -> usize {
        unsafe { self.above.offset_from(self.head) as usize }
    }

    /// Free capacity below `head`.
    #[inline]
    pub fn room_left(&self) -> usize {
        unsafe { self.head.offset_from(self.end) as usize }
    }

    /// **Unchecked**: set depth directly (0 ≤ `idx` ≤ capacity).
    /// # Safety
    /// - the index must be in the stacks range
    /// - this now allows bad reads but non would be excuted automatically
    #[inline]
    pub unsafe fn set_write_index(&mut self, idx: usize) { unsafe {
        self.head = self.above.sub(idx);
    }}

    #[inline]
    pub fn len(&self) -> usize {
        self.write_index()
    }

    #[inline]
    pub fn is_empty(&self) ->bool{
        self.len()==0
    }

/*──────────────────── push / pop ───────────────────────*/
    #[inline]
    pub fn push(&mut self, v: T) -> Result<(), T> {
        if self.room_left() == 0 {
            return Err(v);
        }
        unsafe {
            self.head = self.head.sub(1);
            self.head.write(v);
        }
        Ok(())
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.write_index() == 0 {
            return None;
        }
        unsafe {
            let v = self.head.read();
            self.head = self.head.add(1);
            Some(v)
        }
    }

    #[inline]
    pub fn push_n<const N: usize>(&mut self, arr: [T; N]) -> Result<(), [T; N]> {
        if self.room_left() < N {
            return Err(arr);
        }
        unsafe {
            self.head = self.head.sub(N);
            (self.head as *mut [T; N]).write(arr);
        }
        Ok(())
    }

    #[inline]
    pub fn pop_n<const N: usize>(&mut self) -> Option<[T; N]> {
        if self.write_index() < N {
            return None;
        }
        unsafe {
            let out = (self.head as *mut [T; N]).read();
            self.head = self.head.add(N);
            Some(out)
        }
    }

    #[inline]
    pub fn push_slice(&mut self, src: &[T]) -> Option<()>
    where
        T: Clone,
    {
        if self.room_left() < src.len() {
            return None;
        }
        unsafe {
            self.head = self.head.sub(src.len());
            let dst = slice::from_raw_parts_mut(self.head, src.len());
            dst.clone_from_slice(src);
        }
        Some(())
    }

    

    #[inline]
    pub fn pop_many<'b>(&'b mut self, n: usize) -> Option<&'b mut [T]>
    where
        T: Copy,
    {
        if self.write_index() < n {
            return None;
        }
        unsafe {
            let p = self.head;
            self.head = self.head.add(n);
            Some(slice::from_raw_parts_mut(p, n))
        }
    }

/*──────────────────── allocs ───────────────────────*/
    /// flushes elements from the stack 
    #[inline]
    pub fn flush(&mut self, len:usize) -> Option<()>{
        if self.write_index() < len {
            return None;
        }
        unsafe{
            //force a drop of these elements
            for x in from_raw_parts_mut(self.head,len){
                ptr::read(x);
            }

            self.head = self.head.add(len);
            Some(())
        }
    }

    /// This frees the memory calling no destructor
    #[inline]
    pub fn free(&mut self, len:usize) -> Option<()>{
        if self.write_index() < len {
            return None;
        }
        unsafe{
            self.head = self.head.add(len);
            Some(())
        }
    }

    /// # Safety
    /// Calling this puts invalid memory on the stack.
    /// Using any read operations on it is UB.
    /// This includes flush; however, free is fine.
    #[inline]
    pub unsafe fn alloc(&mut self, len:usize) -> Option<()>{
        if self.room_left() < len {
            return None;
        }
        self.head = unsafe{self.head.sub(len)};
        Some(())
    }
/*──────────────────── checkpoint ───────────────────────*/

#[inline]
    pub fn check_point(&self)-> StackCheckPoint<T>{
        StackCheckPoint(self.head)
    }

    /// # Safety
    /// nothing points to memory we are currently freeing
    #[inline]
    pub unsafe fn goto_checkpoint(&mut self,check_point:StackCheckPoint<T>){
        self.head=check_point.0;
    }
/*──────────────────── peek helpers ─────────────────────*/
    #[inline]
    pub fn peek<'b>(&'b self) -> Option<&'b T> {
        self.peek_n::<1>().map(|a| &a[0])
    }

    #[inline]
    pub fn peek_n<'b, const N: usize>(&'b self) -> Option<&'b [T; N]> {
        if self.write_index() < N {
            None
        } else {
            unsafe { Some(&*(self.head as *const [T; N])) }
        }
    }

    #[inline]
    pub fn peek_many<'b>(&'b self, n: usize) -> Option<&'b [T]> {
        if self.write_index() < n {
            None
        } else {
            unsafe { Some(slice::from_raw_parts(self.head, n)) }
        }
    }

    #[inline]
    pub fn spot<'b>(&'b mut self, n: usize) -> Option<&'b mut T> {
        if self.write_index() <= n {
            None
        } else {
            unsafe { Some(&mut *self.head.add(n)) }
        }
    }

    #[inline]
    pub fn spot_raw(&mut self, n: usize) -> Option<*mut T> {
        if self.write_index() <= n {
            None
        } else {
            unsafe { Some(self.head.add(n)) }
        }
    }

/*──────────────────── complex handeling ─────────────────────*/
    /// Drop `count` items located `skip` elements below the top.
    pub fn drop_inside(&mut self, skip: usize, count: usize) -> Option<()> {
        if skip + count > self.write_index() {
            return None;
        }
        unsafe {
            let p = self.head.add(skip);
            // Explicitly drop the region [p, p+count)
            for x in from_raw_parts_mut(p, count) {
                ptr::read(x);
            }
            // Move the upper part down
            ptr::copy(self.head, self.head.add(count), skip);
            self.head = self.head.add(count);
        }
        Some(())
    }

    /// (live slice on the left, empty stack on the right)
    #[inline]
    pub fn split<'b>(&'b mut self) -> (&'b mut [T], StackRef<'b, T>) {
        let live = self.write_index();
        let left = unsafe { slice::from_raw_parts_mut(self.head, live) };
        let right = StackRef {
            above: self.head,
            head:  self.head,
            end:   self.end,
            _ph:   PhantomData,
        };
        (left, right)
    }

    /// Raw pointer to live slice + empty right-hand stack (no borrow).
    #[inline]
    pub fn split_raw(&mut self) -> (*mut T, StackRef<'_, T>) {
        let ptr_live = self.head;
        let right = StackRef {
            above: self.head,
            head:  self.head,
            end:   self.end,
            _ph:   PhantomData,
        };
        (ptr_live, right)
    }
}
/*──────────────────── iterator ─────────────────────────*/
impl<T> Iterator for StackRef<'_, T> { type Item = T; fn next(&mut self) -> Option<T> { self.pop() } }


#[test]
fn test_lifo_order() {
    let mut storage = make_storage::<&'static str, 3>();
    let mut stack = StackRef::from_slice(&mut storage);

    stack.push("first").unwrap();
    stack.push("second").unwrap();
    stack.push("third").unwrap();

    assert_eq!(stack.pop(), Some("third"));
    assert_eq!(stack.peek(), Some("second").as_ref());
    assert_eq!(stack.pop(), Some("second"));
    assert_eq!(stack.pop(), Some("first"));
    assert_eq!(stack.pop(), None);
}

#[test]
fn test_peek_n() {
    let mut storage = make_storage::<u32, 5>();
    let mut stack = StackRef::from_slice(&mut storage);

    for v in 1..=5 {
        stack.push(v).unwrap();
    }

    // Top-first order: 5 4 3
    assert_eq!(stack.peek_n::<3>().unwrap(), &[5, 4, 3]);

    assert!(stack.peek_n::<6>().is_none());
    assert!(stack.peek_n::<5>().is_some());

    assert_eq!(stack.pop(), Some(5));
    assert!(stack.peek_n::<3>().is_some());

    assert_eq!(stack.pop(), Some(4));
    assert!(stack.peek_n::<4>().is_none());
}

#[test]
fn test_peek_many() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    for i in 1..=5 {
        stack.push(i).unwrap();
    }

    assert_eq!(stack.peek_many(3).unwrap(), &[5, 4, 3]);
    assert!(stack.peek_many(6).is_none());
    assert_eq!(stack.peek_many(5).unwrap(), &[5, 4, 3, 2, 1]);

    stack.pop();
    stack.pop();
    assert!(stack.peek_many(4).is_none());
    assert_eq!(stack.peek_many(3).unwrap(), &[3, 2, 1]);
}

#[test]
fn test_push_n_and_pop_n_success() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    let arr1 = [10, 20];
    let arr2 = [30, 40, 50];

    stack.push_n(arr1).unwrap();
    stack.push_n(arr2).unwrap();

    assert_eq!(stack.pop_n::<3>().unwrap(), [30, 40, 50]);
    assert_eq!(stack.pop_n::<2>().unwrap(), [10, 20]);
    assert!(stack.pop_n::<1>().is_none());
}

#[test]
fn test_push_n_overflow() {
    let mut storage = make_storage::<u32, 4>();
    let mut stack = StackRef::from_slice(&mut storage);

    let ok = [1, 2];
    let fail = [3, 4, 5];

    assert!(stack.push_n(ok).is_ok());
    assert_eq!(stack.push_n(fail), Err(fail));
}

#[test]
fn test_pop_n_underflow() {
    let mut storage = make_storage::<u32, 3>();
    let mut stack = StackRef::from_slice(&mut storage);

    stack.push(1).unwrap();
    assert!(stack.pop_n::<2>().is_none());
    assert!(stack.pop_n::<1>().is_some());
    assert!(stack.pop_n::<1>().is_none());
}

#[test]
fn test_mixed_push_pop_n() {
    let mut storage = make_storage::<u32, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    stack.push_n([1, 2, 3]).unwrap();
    stack.push(4).unwrap();
    stack.push_n([5, 6]).unwrap();

    assert_eq!(stack.pop_n::<2>(), Some([5, 6]));
    assert_eq!(stack.pop(), Some(4));
    assert_eq!(stack.pop_n::<3>(), Some([1, 2, 3]));
    assert!(stack.pop().is_none());
}

#[test]
fn test_slice_conversion_basic() {
    let mut storage = make_storage::<u32, 4>();
    let mut stack = StackRef::from_slice(&mut storage);

    assert_eq!(stack.pop(), None);

    stack.push(10).unwrap();
    stack.push(20).unwrap();

    let idx = stack.write_index();
    assert_eq!(idx, 2);

    let slice = stack.to_slice();
    let mut stack = StackRef::from_slice(slice);
    unsafe { stack.set_write_index(idx) };

    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), None);
}

#[test]
fn test_split_stack() {
    let mut storage = make_storage::<u32, 6>();
    let mut original = StackRef::from_slice(&mut storage);

    original.push(1).unwrap();
    original.push(2).unwrap();
    original.push(3).unwrap();

    let (left, mut right) = original.split();

    assert_eq!(left, [3, 2, 1]);  // top-first
    assert_eq!(right.pop(), None);

    right.push(10).unwrap();
    assert_eq!(right.pop(), Some(10));
}

#[test]
fn test_push_slice_success_and_error() {
    let mut storage = make_storage::<u32, 5>();
    let mut stack = StackRef::from_slice(&mut storage);

    let input1 = [1, 2, 3];
    assert_eq!(stack.push_slice(&input1), Some(()));
    assert_eq!(stack.peek_many(3), Some(&[1, 2, 3][..]));

    assert_eq!(stack.push_slice(&[4, 5, 6]), None);
    stack.push_slice(&[4, 5]).unwrap();

    assert_eq!(stack.write_index(), 5);
    assert!(stack.push_slice(&[99]).is_none());

    stack.pop().unwrap();
    assert!(stack.push_slice(&[99, 66]).is_none());

    stack.pop().unwrap();
    assert!(stack.push_slice(&[99, 66, 11]).is_none());
}

#[test]
fn test_weird_write_error() {
    let mut storage = make_storage::<i64, 6>();
    let mut stack = StackRef::from_slice(&mut storage);

    stack.push_slice(&[2]).unwrap();
    stack.push_n([1]).unwrap();

    stack.push_slice(&[2, 3]).unwrap();
    stack.push_n([2]).unwrap();

    assert!(stack.push_slice(&[1, 2, 3]).is_none());
}

#[test]
fn test_full_usage() {
    let mut data = [10, 20, 30, 40, 50, 60];
    let mut stack = StackRef::new_full(&mut data);

    assert_eq!(stack.write_index(), 6);
    assert_eq!(stack.room_left(), 0);

    // Top item (down-growing stack) is 10
    assert_eq!(stack.peek(), Some(&10));

    assert_eq!(stack.spot(2), Some(&mut 30));

    // Drop (40, 50)
    stack.drop_inside(3, 2).unwrap();
    assert_eq!(stack.room_left(), 2);

    // Pop remaining items in LIFO order
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.room_left(), 4);

    assert_eq!(stack.pop(), Some(30));
    assert_eq!(stack.pop(), Some(60));
    assert_eq!(stack.room_left(), 6);

    assert_eq!(stack.pop(), None);

    stack.push(77).unwrap();
    assert_eq!(stack.peek(), Some(&77));
    stack.push(77).unwrap();

    stack.pop_many(4).ok_or(()).unwrap_err();
    stack.pop_many(2).unwrap();
}


#[test]
fn test_drop_in() {
    let mut data = [10, 20, 30, 40, 50, 60];
    let mut stack = StackRef::new_full(&mut data);

    assert_eq!(stack.write_index(), 6);
    assert_eq!(stack.room_left(), 0);

    // Top item (down-growing stack) is 10
    assert_eq!(stack.peek(), Some(&10));

    // Drop (30, 40, 50)
    stack.drop_inside(2, 3).unwrap();
    assert_eq!(stack.room_left(), 3);

    // Pop remaining items in LIFO order
    assert_eq!(stack.pop(), Some(10));
    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.room_left(), 5);

    assert_eq!(stack.pop(), Some(60));
    assert_eq!(stack.room_left(), 6);

    assert_eq!(stack.pop(), None);

    stack.push(77).unwrap();
    assert_eq!(stack.peek(), Some(&77));
    stack.push(77).unwrap();

    stack.pop_many(4).ok_or(()).unwrap_err();
    stack.pop_many(2).unwrap();
}

#[test]
fn test_drop_inside_skip_zero_should_remove_top_items() {
    // stack top-to-bottom: 10 20 30 40
    let mut data  = [10, 20, 30, 40];
    let mut stack = StackRef::new_full(&mut data);

    // Ask to drop the top two (10,20)
    stack.drop_inside(0, 2).unwrap();

    // ── Expected ─────────────────────────
    // write_index()            == 2
    // subsequent pops: 30 then 40
    // ── Actual with current impl ────────
    // write_index()            is still 4
    // pops start with 10       ← not removed!

    assert_eq!(stack.write_index(), 2, "top items were not removed");
    assert_eq!(stack.pop(),        Some(30));
    assert_eq!(stack.pop(),        Some(40));
    assert_eq!(stack.pop(),        None);
}

#[test]
fn test_zero_capacity_stack() {
    use core::mem::MaybeUninit;

    let mut storage: [MaybeUninit<u32>; 0] = [];
    let mut stack = StackRef::from_slice(&mut storage);

    assert_eq!(stack.write_index(), 0);
    assert_eq!(stack.room_left(),   0);

    assert!(stack.pop().is_none());
    assert_eq!(stack.push(123), Err(123));

    let (slice, mut stack2) = stack.split();

    assert_eq!(slice.len(),0);
    assert_eq!(stack2.write_index(), 0);
    assert_eq!(stack2.room_left(),   0);

    assert!(stack2.pop().is_none());
    assert_eq!(stack2.push(123), Err(123));
}
