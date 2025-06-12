#ifndef STACK_H
#define STACK_H

#include "config.h"
#include "ctypes.h"
#include "errors.h"

#define STACK_LIT(buffer,n) (Stack){buffer+n-1,(intptr_t)buffer -sizeof(Word),(intptr_t)(buffer+n-1)}
#define STACK_REST(stack) stack.cur = (Word*)stack.end;

#define STACK_EMPTY(stack) ((intptr_t)stack.cur==stack.end)

#define STACK_ALLOC(n) stack_alloc(vm,&vm->data_stack,(intptr_t)n)
#define STACK_FREE(n) stack_free(vm,&vm->data_stack,(intptr_t)n)

#define PARAM_DROP(n) stack_free(vm,&vm->param_stack,(intptr_t)n)


#define SPOT(n) *(vm->param_stack.cur+(n))
#define DATA_SPOT(n) *(vm->data_stack.cur+(n))

#define PUSH(p) { *stack_alloc(vm,&vm->param_stack,1)=p;}
#define POP() *stack_free(vm,&vm->param_stack,1)

inline Word* stack_alloc(VM* vm,Stack* stack,intptr_t count){
	intptr_t new_cur = (intptr_t)(stack->cur)-count*sizeof(Word);
	
#ifndef UNCHECKED_STACK_OVERFLOW
	if(new_cur < stack->below)	
		panic(vm,STACK_OVERFLOW);
#endif

	stack->cur = (Word*) new_cur;
	return stack->cur;
}

inline Word* stack_free(VM* vm,Stack* stack,intptr_t count){
	intptr_t new_cur = (intptr_t)(stack->cur)+count*sizeof(Word);
#ifdef DEBUG_MODE
	if(new_cur > stack->end)
		panic(vm,STACK_UNDERFLOW);
#endif
	Word* ans = stack->cur;
	stack->cur = (Word*) new_cur;
	return ans;
}

#endif // STACK_H

