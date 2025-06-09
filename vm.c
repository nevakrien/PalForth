#include "vm.h"
#include "stdbool.h"
#include "utils.h"

static inline Word* stack_alloc(Stack* stack,int count){
	intptr_t new_cur = (intptr_t)(stack->cur)+count*sizeof(Word);
	
#ifndef UNCHECKED_STACK_OVERFLOW
	if(new_cur > stack->end)
		return NULL;
#endif

	Word* ans = stack->cur;
	stack->cur = (Word*) new_cur;
	return ans;
}

static inline bool stack_free(Stack* stack,int count){
	intptr_t new_cur = (intptr_t)(stack->cur)-count*sizeof(Word);
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
	TODO;\
}\

#define STACK_ALLOC(n) {\
	if(!stack_alloc(&vm->stack,n))\
		PANIC("stack overflow");\
}\

#define STACK_FREE(n) {\
	if(!stack_free(&vm->stack,n))\
		PANIC("stack underflow");\
}\

#define SPOT(n) *(vm->stack.cur-n)

#ifdef TEST
static void test_inner(VM* vm){
	STACK_ALLOC(5)
	SPOT(1)=(void*) 2;
	STACK_FREE(5)
}

void test_vm(){
	void* buffer[10];
	VM vm = {0};
	vm.stack = STACK_LIT(buffer,10);
	test_inner(&vm);
}
#endif //TEST
