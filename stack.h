#ifndef STACK_H
#define STACK_H

#include "config.h"
#include "ctypes.h"
#include "errors.h"

#define STACK_LIT(buffer,n) (Stack){buffer+n-1,(intptr_t)buffer -sizeof(Word),(intptr_t)(buffer+n-1)}
#define STACK_REST() vm->stack.cur = (Word*)vm->stack.end;
#define STACK_EMPTY() ((intptr_t)vm->stack.cur==vm->stack.end)

#define STACK_ALLOC(n) stack_alloc(vm,n)
#define STACK_FREE(n) stack_free(vm,n)
#define SPOT(n) *(vm->stack.cur+(n))
#define PUSH(p) { *STACK_ALLOC(1)=p;}
#define POP() *STACK_FREE(1)

inline Word* stack_alloc(VM* vm,int count){
	intptr_t new_cur = (intptr_t)(vm->stack.cur)-count*sizeof(Word);
	
#ifndef UNCHECKED_STACK_OVERFLOW
	if(new_cur < vm->stack.below)	
		panic(vm,STACK_OVERFLOW);
#endif

	vm->stack.cur = (Word*) new_cur;
	return vm->stack.cur;
}

inline Word* stack_free(VM* vm,int count){
	intptr_t new_cur = (intptr_t)(vm->stack.cur)+count*sizeof(Word);
#ifdef DEBUG_MODE
	if(new_cur > vm->stack.end)
		panic(vm,STACK_UNDERFLOW);
#endif
	Word* ans = vm->stack.cur;
	vm->stack.cur = (Word*) new_cur;
	return ans;
}

#endif // STACK_H

