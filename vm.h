#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"


inline Code* excute_code(Code* code,VM* vm){
	if(code->xt){
		return code->xt(code,vm);
	}

	VM_LOG("excuting colon")


	Code* current = code->code_start;
	for(;;current++){
		current=excute_code(current,vm);
		if(current==NULL)
			break;

	}

	return current;
}

inline Code* pick(Code* code,VM* vm){
	VM_LOG("excuting pick")

	PUSH(SPOT((intptr_t)code->code_start));
	return code;
}

inline Code* branch(Code* code,VM* vm){
	VM_LOG("excuting branch")
	
	if(*(palbool_t*)POP()){
		return code->code_start;
	}

	return code;
}

inline Code* jump(Code* code,VM* vm){
	VM_LOG("excuting jump")

	return *(Code**)POP();
}

inline Code* end(Code* code,VM* vm){
	VM_LOG("excuting end")

	return NULL;
}


#ifdef TEST
void test_vm();
#endif

#endif // VM_H

