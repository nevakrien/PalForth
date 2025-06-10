#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"


inline Code* excute_code(Code* code,VM* vm){
	if(code->xt){
		code->xt(code,vm);
		return code;
	}

	Code* current = code->code_start;
	for(;current!=NULL;current++){
		current=excute_code(current,vm);
	}

	return code;
}


#ifdef TEST
void test_vm();
#endif

#endif // VM_H

