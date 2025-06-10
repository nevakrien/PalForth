#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"
#include "string.h"


Code* excute_code(VM* vm,Code* code);


Code* inject(Code* code,VM* vm);
Code* frame_alloc(Code* code,VM* vm);
Code* frame_free(Code* code,VM* vm);
Code* pick(Code* code,VM* vm);
Code* push_local(Code* code,VM* vm);
Code* push_var(Code* code,VM* vm);


Code* branch(Code* code,VM* vm);
Code* jump(Code* code,VM* vm);
Code* call_dyn(Code* code,VM* vm);
Code* ret(Code* code,VM* vm);

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


#ifdef TEST
void test_vm();
#endif

#endif // VM_H

