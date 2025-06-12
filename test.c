#include "vm.h"
#include "cutf8.h"

void test_stack(VM* vm,size_t size){    
    PUSH((Word) 5);
    assert(POP()==(Word) 5);

    PUSH((Word) 5);
    PUSH((Word) 3);
    assert(POP()==(Word) 3);
    assert(POP()==(Word) 5);

    STACK_ALLOC(size);
    DATA_SPOT(1)=(void*) 2;
    DATA_SPOT(size)=(void*) 2;
    STACK_FREE(size);

    printf("stack actions: passed\n");

#ifdef DEBUG_MODE
    //check crash
    jmp_buf jmp;
    bool jumped = false;

    vm->panic_handler = &jmp;

    if (setjmp(jmp) == 0) {
        PARAM_DROP(1);
        fprintf(stderr, "ERROR: longjmp did not trigger on underflow\n");
        exit(1);
    } else {
        // We landed here via longjmp
        jumped = true;
        STACK_REST(vm->param_stack);
    }

    vm->panic_handler = NULL;

    assert(jumped && "longjmp was not triggered when expected");
    printf("stack exceptions: passed\n");

#endif //DEBUG_MODE
}

void test_core_ops(VM* vm,size_t size){
    void* cannary = (void*) 321;
    

    //test simple pick

    Code dup_code[] = {
        (Code){pick,(Code*)0},
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
        (Code){push_local,(Code*)0},
        (Code){pick,(Code*)1},
        (Code){inject,(Code*)(5*sizeof(Word))},
        (Code){param_drop,(Code*)1},

        //inject back out
        (Code){pick,(Code*)1},
        (Code){push_local,(Code*)0},
        (Code){inject,(Code*)(5*sizeof(Word))},
        (Code){param_drop,(Code*)1},


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

    printf("tgt = %p src = %p \n",tgt,src);

    PUSH(tgt);
    PUSH(src);
    excute_code(vm,&frame_word);
    assert(memcmp(src,tgt,sizeof(src))==0);

    PARAM_DROP(2);

    printf("frame injections: passed\n");



    //test branches

    assert(STACK_EMPTY(vm->param_stack));


    Code maybe_dup[] = {
        (Code){branch,(Code*)1},
        (Code){pick,(Code*)0},
        (Code){ret},
    };

    Code maybe_dup_word = {
        NULL,
        maybe_dup,
    };

    palbool_t b = 1;

    PUSH(&b);
    assert(excute_code(vm,&maybe_dup_word)==NULL);
    assert(STACK_EMPTY(vm->param_stack));


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
        (Code){pick, (Code*)1},
        (Code){pick, (Code*)1},
        (Code){NULL}, // to be filled dynamically
        (Code){param_drop,(Code*)1},
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

void test_bool_and_compare(VM* vm, size_t size) {
    Code cmp_code[] = {
        (Code){pick, (Code*)2},
        (Code){pick, (Code*)2},
        (Code){pick, (Code*)2},
        (Code){NULL},
        (Code){param_drop,(Code*)1},
        (Code){ret},
    };

    Code cmp_word = {
        NULL,
        cmp_code,
    };

    palint_t a = 5;
    palint_t b = 3;
    palbool_t result;

    // Push once: target, source1, source2
    PUSH(&result);  // target (write result here)
    PUSH(&a);
    PUSH(&b);

    // === EQ ===
    {
        a = 5; b = 5;
        cmp_code[3].xt = int_eq;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    // === NEQ ===
    {
        a = 5; b = 6;
        cmp_code[3].xt = int_neq;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    // === LT ===
    {
        a = 2; b = 3;
        cmp_code[3].xt = int_smaller;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    // === GT ===
    {
        a = 9; b = 4;
        cmp_code[3].xt = int_bigger;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    // === LE ===
    {
        a = 4; b = 4;
        cmp_code[3].xt = int_le;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    // === GE ===
    {
        a = 7; b = 2;
        cmp_code[3].xt = int_ge;
        excute_code(vm, &cmp_word);
        assert(result == 1);
    }

    PARAM_DROP(3);

    // === LOGICAL OPS ===
    Code bool_code[] = {
        (Code){pick, (Code*)1},
        (Code){pick, (Code*)1},
        (Code){NULL},
        (Code){param_drop,(Code*)1},
        (Code){ret},
    };

    Code bool_word = {
        NULL,
        bool_code,
    };

    palbool_t x = 1;
    palbool_t y = 0;

    PUSH(&x);
    PUSH(&y);

    // === AND ===
    {
        x = 1; y = 1;
        bool_code[2].xt = bool_and;
        excute_code(vm, &bool_word);
        assert(x);
    }

    // === OR ===
    {
        x = 0; y = 1;
        bool_code[2].xt = bool_or;
        excute_code(vm, &bool_word);
        assert(x);
    }

    // === XOR ===
    {
        x = 1; y = 1;
        bool_code[2].xt = bool_xor;
        excute_code(vm, &bool_word);
        assert(!x);
    }

    PARAM_DROP(2);

    // === NOT === (manual op: unary)
    {
        x = 1;
        Code not_code[] = {
            (Code){bool_not},
            (Code){param_drop,(Code*)1},
            (Code){ret},
        };
        Code not_word = {
            NULL,
            not_code,
        };

        PUSH(&x);
        excute_code(vm, &not_word);
        assert(!x);
    }
}



void test_vm(){
    void* buffer[10];
    void* buffer2[10];
    VM vm = {0};
    vm.data_stack = STACK_LIT(buffer,10);
    vm.param_stack = STACK_LIT(buffer2,10);

    test_stack(&vm,10);
    test_core_ops(&vm,10);
    test_arithmetics(&vm,10);
    test_bool_and_compare(&vm,10);
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