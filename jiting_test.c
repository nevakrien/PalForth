#include "vm.h"

void test_interp_equiv_manual(VM* vm) {
    Word src[5] = {(Word*)1, (Word*)3, (Word*)1, (Word*)1, (Word*)-1};
    Word tgt[5] = {0};

    PUSH(tgt);
    PUSH(src);

    Code dummy;

    dummy.xt = frame_alloc;
    dummy.first_const = (Code*)5;
    frame_alloc(&dummy, vm);

    dummy.xt = push_local;
    dummy.first_const = (Code*)0;
    push_local(&dummy, vm);

    dummy.xt = pick;
    dummy.first_const = (Code*)1;
    pick(&dummy, vm);

    dummy.xt = inject;
    dummy.first_const = (Code*)(5 * sizeof(Word));
    inject(&dummy, vm);

    dummy.xt = param_drop;
    dummy.first_const = (Code*)1;
    param_drop(&dummy, vm);

    dummy.xt = pick;
    dummy.first_const = (Code*)1;
    pick(&dummy, vm);

    dummy.xt = push_local;
    dummy.first_const = (Code*)0;
    push_local(&dummy, vm);

    dummy.xt = inject;
    dummy.first_const = (Code*)(5 * sizeof(Word));
    inject(&dummy, vm);

    dummy.xt = param_drop;
    dummy.first_const = (Code*)1;
    param_drop(&dummy, vm);

    dummy.xt = frame_free;
    dummy.first_const = (Code*)5;
    frame_free(&dummy, vm);

    assert(memcmp(src, tgt, sizeof(src)) == 0);
    PARAM_DROP(2);

    printf("[MANUAL INTERPRETER] frame injections: passed\n");
}

int main() {
    void* buffer[10];
    void* buffer2[10];
    VM vm = {0};
    vm.data_stack = STACK_LIT(buffer, 10);
    vm.param_stack = STACK_LIT(buffer2, 10);

    test_interp_equiv_manual(&vm);
    return 0;
}
