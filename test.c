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

    vm->panic_handler = NULL;

    assert(jumped && "longjmp was not triggered when expected");
    printf("stack exceptions: passed\n");

}

void test_core_ops(VM* vm,size_t size){
    void* cannary = (void*) 321;
    

    //test simple pick

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


    assert(size>=10);

    Code frame_code[]= {
        //prelude
        (Code){frame_alloc,(Code*)5},
        
        //inject to the stack
        (Code){push_local,(Code*)5},
        (Code){pick,(Code*)7},//top arg 5frame+1pushed+1offset
        (Code){inject,(Code*)(5*sizeof(Word))},

        //inject back out
        (Code){pick,(Code*)7},//bottom arg 5frame+2offset
        (Code){push_local,(Code*)6},
        (Code){inject,(Code*)(5*sizeof(Word))},

        //epilogue
        (Code){frame_free,(Code*)5},
        (Code){ret},
    };

    Code frame_word = {
        NULL,
        frame_code,
    };

    Word src[5] = {(Word*)1,(Word*)3,(Word*)1,(Word*)1,(Word*)-1};
    Word tgt[5] = {0};

    PUSH(tgt);
    PUSH(src);
    excute_code(vm,&frame_word);
    assert(memcmp(src,tgt,sizeof(src))==0);

    STACK_FREE(2);

    printf("frame injections: passed\n");



    //test branches

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

    printf("branch if: passed\n");


}

void test_arithmetics(VM* vm, size_t size) {
    Code arith_code[] = {
        (Code){pick, (Code*)2},
        (Code){pick, (Code*)2},
        (Code){NULL}, // to be filled dynamically
        (Code){ret},
    };

    Code arith_word = {
        NULL,
        arith_code,
    };

    palint_t a, b;

    // Push once
    PUSH(&a);
    PUSH(&b);

    // === ADD ===
    {
        a = 1;
        b = 2;
        palint_t c = a + b;
        arith_code[2].xt = int_add;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === SUB ===
    {
        a = 5;
        b = 3;
        palint_t c = a - b;
        arith_code[2].xt = int_sub;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === MUL ===
    {
        a = 6;
        b = 7;
        palint_t c = a * b;
        arith_code[2].xt = int_mul;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === DIV ===
    {
        a = 20;
        b = 5;
        palint_t c = a / b;
        arith_code[2].xt = int_div;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === SHL ===
    {
        a = 3;
        b = 2;
        palint_t c = a << b;
        arith_code[2].xt = int_shl;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === SHR ===
    {
        a = 16;
        b = 2;
        palint_t c = a >> b;
        arith_code[2].xt = int_shr;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === AND ===
    {
        a = 0b1100;
        b = 0b1010;
        palint_t c = a & b;
        arith_code[2].xt = int_and;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === OR ===
    {
        a = 0b1100;
        b = 0b1010;
        palint_t c = a | b;
        arith_code[2].xt = int_or;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === XOR ===
    {
        a = 0b1100;
        b = 0b1010;
        palint_t c = a ^ b;
        arith_code[2].xt = int_xor;
        excute_code(vm, &arith_word);
        assert(a == c);
    }

    // === MOD ===
    {
        a = 17;
        b = 5;
        palint_t c = a % b;
        arith_code[2].xt = int_mod;
        excute_code(vm, &arith_word);
        assert(a == c);
    }
}



void test_vm(){
    void* buffer[10];
    VM vm = {0};
    vm.stack = STACK_LIT(buffer,10);

    test_stack(&vm,10);
    test_core_ops(&vm,10);
    test_arithmetics(&vm,10);
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