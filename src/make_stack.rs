use crate::stack::StackRef;

#[cfg(all(unix, target_os = "linux", feature = "std"))]
pub unsafe fn new_os_stack<T>() -> Option<StackRef<'static, T>> { unsafe {
    use libc::{mmap, sysconf, _SC_PAGESIZE, MAP_ANON, MAP_PRIVATE, MAP_STACK, MAP_GROWSDOWN, PROT_READ, PROT_WRITE};
    use std::ptr::null_mut;

    let _page_size = sysconf(_SC_PAGESIZE) as usize;
    let size = 1 << 20; // 1 MiB

    let ptr = mmap(
        null_mut(),
        size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANON | MAP_STACK | MAP_GROWSDOWN,
        -1,
        0,
    );
    if ptr == libc::MAP_FAILED {
        return None;
    }

    // let _ = mprotect(ptr, page_size, PROT_NONE); // guard page manually

    let typed_ptr = ptr as *mut T;
    let total_elems = size / std::mem::size_of::<T>();

    Some(StackRef {
        above: typed_ptr.add(total_elems),
        head:  typed_ptr.add(total_elems),
        end:   core::ptr::null_mut(), // dynamic expansion
        _ph: std::marker::PhantomData,
    })
}}

#[cfg(feature = "flaky_tests")]
#[cfg(all(unix, target_os = "linux", feature = "std"))]
#[test]
fn test_stack_grows_down_on_access() {
    use std::ptr;

    unsafe {
        let stack = new_os_stack::<u8>().expect("Failed to allocate mmap stack");
        assert!(!stack.above.is_null());

        // Starting at head, walk down one page at a time
        let mut sp = stack.head;
        let page_size = libc::sysconf(libc::_SC_PAGESIZE) as isize;
        let pages = 1024; // try 16 pages below head

        for _i in 1..=pages {
            sp = sp.offset(-(page_size as isize));
            ptr::write_volatile(sp, 0xCC); // force memory access to trigger fault handling
        }

        // Print success
        println!("Stack underflowed {} pages successfully", pages-256);
    }
}


#[cfg(all(unix, not(target_os = "linux"), feature = "std"))]
pub unsafe fn new_os_stack<T>() -> Option<StackRef<'static, T>> {unsafe {
    use libc::{mmap, sysconf, _SC_PAGESIZE, MAP_ANON, MAP_PRIVATE, MAP_STACK, PROT_READ, PROT_WRITE};
    use std::ptr::null_mut;

    let page_size = sysconf(_SC_PAGESIZE) as usize;
    let size = 1 << 20;

    let ptr = mmap(
        null_mut(),
        size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANON | MAP_STACK,
        -1,
        0,
    );
    if ptr == libc::MAP_FAILED {
        return None;
    }

    let typed_ptr = ptr as *mut T;
    let total_elems = size / std::mem::size_of::<T>();
    let guard_elems = page_size / std::mem::size_of::<T>();

    Some(StackRef {
        above: typed_ptr.add(total_elems),
        head:  typed_ptr.add(total_elems),
        end:   typed_ptr.add(guard_elems),
        _ph: std::marker::PhantomData,
    })
}}



#[cfg(all(windows, feature = "std"))]
pub unsafe fn new_os_stack<T>() -> Option<StackRef<'static, T>> {unsafe {
    use std::ptr::null_mut;
    use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_NOACCESS};

    let size = 1 << 20; // 1 MiB
    let guard_size = 4096; // 1 page

    let ptr = VirtualAlloc(
        null_mut(),
        size,
        MEM_RESERVE | MEM_COMMIT,
        PAGE_READWRITE,
    ) as *mut T;
    if ptr.is_null() {
        return None;
    }

    let _ = VirtualProtect(ptr as *mut _, guard_size, PAGE_NOACCESS, &mut 0);

    let total_elems = size / std::mem::size_of::<T>();
    let guard_elems = guard_size / std::mem::size_of::<T>();

    Some(StackRef {
        above: ptr.add(total_elems),
        head:  ptr.add(total_elems),
        end:   ptr.add(guard_elems),
        _ph: std::marker::PhantomData,
    })
}}
