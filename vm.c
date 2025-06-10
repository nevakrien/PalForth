#include "vm.h"
#include "utils.h"

extern inline Word* stack_alloc(VM* vm,int count);
extern inline Word* stack_free(VM* vm,int count);

void panic(VM* vm,long code){
	if(vm->panic_handler)
		longjmp(*vm->panic_handler,code);
	else
		exit(code);
}

#define GOTO(x) vm->pc=x;
#define PANIC(code) panic(vm,code)

#define STACK_ALLOC(n) stack_alloc(vm,n)
#define STACK_FREE(n) stack_free(vm,n)

#define SPOT(n) *(vm->stack.cur-(n))
#define PUSH(p) { *STACK_ALLOC(1)=p;}
#define POP() *STACK_FREE(1)

#ifdef TEST
static void test_inner(VM* vm,size_t size){
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
	test_inner(&vm,5);
}
#endif //TEST
