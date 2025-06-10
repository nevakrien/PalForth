#include "vm.h"
#include "utils.h"

extern inline Word* stack_alloc(VM* vm,int count);
extern inline Word* stack_free(VM* vm,int count);
Code* excute_code(VM* vm,Code* code){
	if(code->xt){
		return code->xt(code,vm);
	}

	VM_LOG("excuting colon")


	Code* current = code->code_start;
	for(;;current++){
		current=excute_code(vm,current);
		if(current==NULL)
			break;

	}

	return current;
}

extern inline Code* inject(Code* code,VM* vm);
extern inline Code* frame_alloc(Code* code,VM* vm);
extern inline Code* frame_free(Code* code,VM* vm);
extern inline Code* push_local(Code* code,VM* vm);

extern inline Code* pick(Code* code,VM* vm);
extern inline Code* branch(Code* code,VM* vm);
extern inline Code* jump(Code* code,VM* vm);
extern inline Code* call_dyn(Code* code,VM* vm);
extern inline Code* ret(Code* code,VM* vm);

void panic(VM* vm,long code){
	if(vm->panic_handler)
		longjmp(*vm->panic_handler,code);
	else
		exit(code);
}
