#include "vm.h"
#include "cutf8.h"

void test_stack(VM* vm,size_t size){    
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
        vm->stack.cur=(Word*)vm->stack.base;
    }

    assert(jumped && "longjmp was not triggered when expected");
    printf("stack exceptions: passed\n");

}

void test_buildins(VM* vm){
    void* cannary = (void*) 321;
    

    Code dup_code[] = {
        (Code){pick,(Code*)1},
        (Code){ret},
    };

    Code dup_word = {
        NULL,
        dup_code,
    };

    PUSH(cannary);

    assert(excute_code(vm,&dup_word)==NULL);

    assert(POP()==cannary);
    assert(POP()==cannary);

    printf("pick happy: passed\n");


    assert((intptr_t)vm->stack.cur==vm->stack.base);


    Code maybe_dup[] = {
        (Code){branch,(Code*)1},
        (Code){pick,(Code*)1},
        (Code){ret},
    };

    Code maybe_dup_word = {
        NULL,
        maybe_dup,
    };

    palbool_t b = 1;

    PUSH(&b);
    assert(excute_code(vm,&maybe_dup_word)==NULL);
    assert((intptr_t)vm->stack.cur==vm->stack.base);

    PUSH(cannary);
    b=0;
    PUSH(&b);
    assert(excute_code(vm,&maybe_dup_word)==NULL);
    assert(POP()==cannary);
    assert(POP()==cannary);

    printf("branch happy: passed\n");


}

void test_vm(){
    void* buffer[5];
    VM vm = {0};
    vm.stack = STACK_LIT(buffer,5);
    test_stack(&vm,5);
    test_buildins(&vm);
}

int main(){
	(void)cutf8_get;
    (void)cutf8_put;
    (void)cutf8_copy;
    (void)cutf8_skip;
    (void)cutf8_valid_buff;

    test_vm();
	printf("!!!All Tests Passed!!!\n");
	return 0;
}