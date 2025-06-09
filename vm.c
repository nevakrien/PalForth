#include "vm.h"
#include "stdbool.h"
#include "utils.h"

static inline Word* stack_alloc(Stack* stack,int count){
	intptr_t new_cur = (intptr_t)(stack->cur)+count;
	
#ifndef UNCHECKED_STACK_OVERFLOW
	if(new_cur > stack->end)
		return NULL;
#endif

	Word* ans = stack->cur;
	stack->cur = (Word*) new_cur;
	return ans;
}

static inline bool stack_free(Stack* stack,int count){
	intptr_t new_cur = (intptr_t)(stack->cur)-count;
#ifdef DEBUG_MODE
	if(new_cur < stack->base)
		return 0;
#endif
	stack->cur = (Word*) new_cur;
	return 1;
}

#define GOTO(x) vm->pc=x;
#define PANIC(s) {\
	TODO;\
	GOTO(vm->catch_point);\
	continue;\
}\

#define STACK_ALLOC(n) {\
	if(!stack_alloc(&vm->stack,n))\
		PANIC("stack overflow");\
}\

#define STACK_FREE(n) {\
	if(!stack_free(&vm->stack,n))\
		PANIC("stack underflow");\
}\

void test(VM* vm){
	STACK_ALLOC(5)
	STACK_FREE(5)
}