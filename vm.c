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
	vm->error_point = vm->pc;\
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

#define SPOT(n) *(vm->stack.cur-(n))
#define PUSH(p) { STACK_ALLOC(1); SPOT(1)=p;}
#define POP_TO(p) { STACK_FREE(1); p=SPOT(0); }

#ifdef TEST
static void test_inner(VM* vm,size_t size){
	void* mem = (void*) 5;
	PUSH(mem);
	mem=0;
	POP_TO(mem);
	assert(mem==(void*) 5);

	STACK_ALLOC(size)
	SPOT(1)=(void*) 2;
	SPOT(size-1)=(void*) 2;
	STACK_FREE(size)

}

void test_vm(){
	void* buffer[5];
	VM vm = {0};
	vm.stack = STACK_LIT(buffer,5);
	test_inner(&vm,5);
}
#endif //TEST
