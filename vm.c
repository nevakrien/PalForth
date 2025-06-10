#include "vm.h"
#include "utils.h"

extern inline Word* stack_alloc(VM* vm,int count);
extern inline Word* stack_free(VM* vm,int count);
extern inline Code* excute_code(Code* code,VM* vm);
extern inline Code* pick(Code* code,VM* vm);
extern inline Code* branch(Code* code,VM* vm);
extern inline Code* jump(Code* code,VM* vm);
extern inline Code* end(Code* code,VM* vm);

void panic(VM* vm,long code){
	if(vm->panic_handler)
		longjmp(*vm->panic_handler,code);
	else
		exit(code);
}
