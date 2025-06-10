#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"
#include "string.h"


Code* excute_code(VM* vm,Code* code);

inline Code* inject(Code* code,VM* vm){
	VM_LOG("excuting inject")

	Word source = POP();
	Word target = POP();
	memmove(target,source,(intptr_t)code->code_start);

	return code;
}

inline Code* frame_alloc(Code* code,VM* vm){
	VM_LOG("excuting set_sp")

	STACK_ALLOC((intptr_t)code->code_start);
	return code;
}

inline Code* frame_free(Code* code,VM* vm){
	VM_LOG("excuting set_sp")

	STACK_FREE((intptr_t)code->code_start);
	return code;
}

inline Code* push_local(Code* code,VM* vm){
	VM_LOG("excuting pick")

	PUSH(&SPOT((intptr_t)code->code_start));
	return code;
}

inline Code* pick(Code* code,VM* vm){
	VM_LOG("excuting pick")

	PUSH(SPOT((intptr_t)code->code_start));
	return code;
}

inline Code* branch(Code* code,VM* vm){
	VM_LOG("excuting branch")
	
	if(*(palbool_t*)POP()){
		return code+(intptr_t)code->code_start;
	}

	return code;
}

inline Code* jump(Code* code,VM* vm){
	VM_LOG("excuting jump")

	return code+(intptr_t)code->code_start;
}

inline Code* call_dyn(Code* code,VM* vm){
	VM_LOG("excuting call_dyn")

	return *(Code**)POP();
}

inline Code* ret(Code* code,VM* vm){
	VM_LOG("excuting end")

	return NULL;
}


#ifdef TEST
void test_vm();
#endif

#endif // VM_H

