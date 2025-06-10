#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"
#include <setjmp.h>

typedef struct vm VM;
typedef struct code Code;
typedef int (*XT)(VM*,Code*);
typedef void* Word;

typedef struct{
	Word* cur;
	intptr_t base; //avoid weird UB rules on comparing
	intptr_t end; //1 above last valid address
} Stack;
#define STACK_LIT(buffer,n) (Stack){buffer,(intptr_t)buffer,(intptr_t)(buffer+n)}


void panic(VM* vm,long code);

struct code {
	XT xt;
	//data goes here
};

#define CODE_DATA(code) ((void *)((char *)(code) + sizeof(Code)))


struct vm {
	Stack stack;
	Code* pc;
	
	Code* catch_point;
	Code* error_point;
	jmp_buf* panic_handler;

	Arena temp_mem;


#ifdef USE_ARENA
	Arena dict_mem; //can have memory shared to other VMs
#endif

};




inline Word* stack_alloc(VM* vm,int count){
	intptr_t new_cur = (intptr_t)(vm->stack.cur)+count*sizeof(Word);
	
#ifndef UNCHECKED_STACK_OVERFLOW
	if(new_cur > vm->stack.end)
		panic(vm,2);
#endif

	Word* ans = vm->stack.cur;
	vm->stack.cur = (Word*) new_cur;
	return ans;
}

inline Word* stack_free(VM* vm,int count){
	intptr_t new_cur = (intptr_t)(vm->stack.cur)-count*sizeof(Word);
#ifdef DEBUG_MODE
	if(new_cur < vm->stack.base)
		panic(vm,3);

#endif
	vm->stack.cur = (Word*) new_cur;
	return vm->stack.cur;
}


#ifdef TEST
void test_vm();
#endif

#endif // VM_H

