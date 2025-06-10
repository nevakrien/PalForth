#include "vm.h"
#include "utils.h"

extern inline Word* stack_alloc(VM* vm,int count);
extern inline Word* stack_free(VM* vm,int count);
extern inline Code* excute_code(Code* code,VM* vm);

void panic(VM* vm,long code){
	if(vm->panic_handler)
		longjmp(*vm->panic_handler,code);
	else
		exit(code);
}

#ifdef TEST
static void test_stack(VM* vm,size_t size){
	void* mem = (void*) 5;
	PUSH(mem);
	mem=0;
	mem=POP();
	assert(mem==(void*) 5);

	STACK_ALLOC(size);
	SPOT(1)=(void*) 2;
	SPOT(size-1)=(void*) 2;
	STACK_FREE(size);

	printf("stack actions: passed\n");


	//check crash
	jmp_buf jmp;
	bool jumped = false;

	vm->panic_handler = &jmp;

	if (setjmp(jmp) == 0) {
		STACK_FREE(1);
		fprintf(stderr, "ERROR: longjmp did not trigger on underflow\n");
		exit(1);
	} else {
		// We landed here via longjmp
		jumped = true;
	}

	assert(jumped && "longjmp was not triggered when expected");
	printf("stack exceptions: passed\n");

}

void test_vm(){
	void* buffer[5];
	VM vm = {0};
	vm.stack = STACK_LIT(buffer,5);
	test_stack(&vm,5);
}
#endif //TEST
