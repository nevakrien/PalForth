#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"
#include "string.h"

//entry point
Code* excute_code(VM* vm,Code* code);

// === BUILD IN COMMANDS === 

//stack managment

// assign ( x → x )
Code* inject(Code* code,VM* vm);

// assign ( x → non-unique x )
Code* inject_non_unique(Code* code,VM* vm);


// allocs ( → )
Code* frame_alloc(Code* code,VM* vm);
Code* frame_free(Code* code,VM* vm);
Code* param_drop(Code* code,VM* vm);

// push ( → ▪x )
Code* pick(Code* code,VM* vm);
Code* push_local(Code* code,VM* vm);
Code* push_var(Code* code,VM* vm);


//control flow ( x → )
Code* branch(Code* code,VM* vm);
Code* call_dyn(Code* code,VM* vm);

//jumps ( → )
Code* jump(Code* code,VM* vm);
Code* ret(Code* code,VM* vm);

//IO ( → )
Code* const_print(Code* code,VM* vm);

// arithmetic ( int → int )
Code* int_add(Code* code, VM* vm);
Code* int_sub(Code* code, VM* vm);
Code* int_mul(Code* code, VM* vm);
Code* int_div(Code* code, VM* vm);
Code* int_mod(Code* code, VM* vm);
Code* int_shl(Code* code, VM* vm);
Code* int_shr(Code* code, VM* vm);
Code* int_and(Code* code, VM* vm);
Code* int_or(Code* code, VM* vm);
Code* int_xor(Code* code, VM* vm);

// comparisons (int int → bool)
Code* int_eq(Code* code, VM* vm);
Code* int_neq(Code* code, VM* vm);
Code* int_smaller(Code* code, VM* vm);
Code* int_bigger(Code* code, VM* vm);
Code* int_le(Code* code, VM* vm);    // suggest you add this
Code* int_ge(Code* code, VM* vm);    // suggest you add this

// logical (mut bool bool → )
Code* bool_and(Code* code, VM* vm);
Code* bool_or(Code* code, VM* vm);
Code* bool_xor(Code* code, VM* vm);

// logical (mut bool → )
Code* bool_not(Code* code, VM* vm);


// === IMPORTANT FUNCTIONS === 

inline void read_palint(palint_t* target,Word source){
	if(CELLS(palint_t)==1&& ALIGN(palint_t)<=ALIGN(Word))
		*target=*(palint_t*)source;
	else
		memcpy(target,source,sizeof(palint_t));
}

inline void write_palint(Word target,palint_t* source){
	if(CELLS(palint_t)==1&& ALIGN(palint_t)<=ALIGN(Word))
		*(palint_t*)target=*source;
	else
		memcpy(target,source,sizeof(palint_t));
}

#endif // VM_H

