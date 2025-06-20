# PalForth
a forth inspired language made for use as a hackble VM 

# The Plan
we hope to get a language that's simple and extremely performant while also allowing arbitrary extensions by user code.
the idea is to eventually use this lower level language as a baseline for a language with a much richer syntax and type system named PAL.

the reason to make a separate VM is that current solutions do not provide a VM that is

1. safe by default
2. boxed
3. interactive
4. allows immediate code

we have forth which checks is interactive and allows immediate code.
we have something like python which is boxed and safe by default and interactive.
there isn't really anything that does all 4.

considering how old forth is it is kind of surprising we don't have any other proper language that allows arbitrary extension and immediate code the same way.

in term of performance we would really like to be fast enough for the average use case which scripting languages like Python and even JavaScript (even with how optimized it is) generally don't fully manage. Ideally code written just in PAL should be enough for pretty much everything that does not require optimizing by reading/writing assembly.

# Scope
right now we are working on version 1 so we just want a basic version that can do most of the common operations and can be build upon. some features would have to be added later but we will still discuss them as their implementation informs the architecture of the VM and especially the type system.

Things which are planned but wont apear in this version of the VM are:
1. threads tasks etc
2. heap 
3. defining buildin words in PalForth

for performance there is no way we would beat JavaScript in all cases without serious serious work as JavaScript can become almost optimal native code for a few small sub-cases. HOWEVER languages like Java and C# can beat JavaScript in most cases because they have a proper type system which allows for much better machine code.

so the goal is to get performance which is faster than a scripting language for many many cases.

# VM engine structure
every VM buildin is simply a function call and some data. this remarkably simple design is directly stolen from eforth with an important twist. we keep our instruction counter available to change in any buildin.
this allows for the addition of arbitrary control structures. this is achieved with the following signature

```Rust
fn(*const Code,&mut Vm<'vm,'_>) -> *const Code
```

which keeps the instruction pointer in registers throughout the entire program. the incrementing is handeled in the main execution loop and buildins need to be aware of this when using control structures.

most instructions would end up returning the input they got which on many architectures ends up being a no op.
annoyingly x86 is an exception but even then adding a
```asm
mov rax,rdi
```

is relatively negligible compared to the cost of moving arguments on the stack.

JITed code will usually inline most of the smaller function calls to avoid all of this cost that comes from calling conventions. 

# Memory Management
PALFORTH is going with a very different approach to memory management than is typically seen in desktop environments. Unlike most desktop oriented languages for us the HEAP is an optional dependency. This means that most pal programs live entirely on the stack and need to manage their memory there.

So the idiomatic way to define something like a dynamic array is with a maximum size capacity and have it exist on the data stack. for some cases it IS possible to define an unbounded array. for instance when reading a file into memory that file can go into 

1. the data stack (since a stack is unbounded and we require no locals)
2. static memory 
3. heap (if available)

if we do choose to put something on the heap then it will be automatically cleaned up. in most cases it is entirely possible to clean it using a simple deconstructor similar to what Rust and C++ do.
however in cases where it is not trivial to tell who is responsible for destroying an object we do employ a GC.

unlike Rust we avoid complex lifetimes in favor of using one of the following:
1. direct values
2. GC or other runtime checks
3. raw pointers (rarely needed like it is in Rust)

# Type system and safety
in general pal uses boxed values for everything, this if done without care would present the need for a GC or the risk for unsoundness on even the most basic functions. however we make 1 simple observation. if every function gets the location to which it needs to write the output. AND if at no point do we hold raw references except for these points.

then all of the lifetimes become trivial and the entire thing is just safe. this is whats done for 99% of functions with a few notable tricky exceptions:

1. indexing operations
2. raw pointers
3. complex reference types such as trees 

for raw pointers we obviously have no choice but to make that code unsafe.
for complex reference types we are willing to sacrifice a GC or some other form of smart pointer.

however for indexing we want to be more clever and this is where we ALLOW a function to return an output variable on the stack which it did not previously get as an input, but instead is DERIVED from the inputs.
these functions are for the most part buildins but in principle it should be possible to write them safely.

the tricky part is that they do in fact have lifetime issues and thus kind of by definition require an internally unsafe implementation unless we had a full Rust like lifetime system (which we on purpose do not)

