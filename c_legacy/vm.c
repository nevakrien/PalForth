#include "vm.h"
#include "utils.h"

extern inline Word* stack_alloc(VM* vm,Stack* stack,intptr_t count);
extern inline Word* stack_free(VM* vm,Stack* stack,intptr_t count);

extern inline void read_palint(palint_t* target,Word source);
extern inline void write_palint(Word target,palint_t* source);

Code* excute_code(VM* vm,Code* code){
	if(code->xt){
		return code->xt(code,vm);
	}

	VM_LOG(puts("excuting colon"))


	Code* current = code->first_const;
	for(;;current++){
		current=excute_code(vm,current);
		if(current==NULL)
			break;

	}

	return current;
}

#define DEFINE_BUILTIN(name, body) \
	Code* name(Code* code, VM* vm) { \
		VM_LOG(puts("executing " #name)); \
		body \
	}

DEFINE_BUILTIN(inject,
	Word source = POP();
	Word target = SPOT(0);
	VM_LOG(printf("target %p source %p\n",target,source));
	memcpy(target, source, (intptr_t)code->first_const);
	return code;
)

DEFINE_BUILTIN(inject_non_unique,
	Word source = POP();
	Word target = SPOT(0);
	VM_LOG(printf("target %p source %p\n",target,source));
	memmove(target, source, (intptr_t)code->first_const);
	return code;
)

DEFINE_BUILTIN(frame_alloc,
	STACK_ALLOC((intptr_t)code->first_const);
	return code;
)

DEFINE_BUILTIN(frame_free,
	STACK_FREE((intptr_t)code->first_const);
	return code;
)

DEFINE_BUILTIN(param_drop,
	PARAM_DROP((intptr_t)code->first_const);
	return code;
)


DEFINE_BUILTIN(push_local,
	VM_LOG(printf("pushing %p\n",&DATA_SPOT((intptr_t)code->first_const)));
	PUSH(&DATA_SPOT((intptr_t)code->first_const));
	return code;
)

DEFINE_BUILTIN(push_var,
	PUSH(code->first_const);
	return code;
)


DEFINE_BUILTIN(pick,
	VM_LOG(printf("pushing %p\n",SPOT((intptr_t)code->first_const)));
	PUSH(SPOT((intptr_t)code->first_const));
	return code;
)

DEFINE_BUILTIN(branch,
	if (*(palbool_t*)POP()) {
		return code + (intptr_t)code->first_const;
	}
	return code;
)

DEFINE_BUILTIN(jump,
	return code + (intptr_t)code->first_const;
)

DEFINE_BUILTIN(call_dyn,
	return *(Code**)POP();
)

DEFINE_BUILTIN(ret,
	return NULL;
)

DEFINE_BUILTIN(const_print,
	printf("%s",(char*)code->first_const);
	return code;
)


#define DEFINE_ARITH(name, op) \
DEFINE_BUILTIN(name, \
	Word source = POP(); \
	Word target = SPOT(0); \
\
	palint_t a;\
	palint_t b;\
	read_palint(&a, target); \
	read_palint(&b, source); \
\
	a = a op b; \
	write_palint(target, &a); \
	return code; \
)

DEFINE_ARITH(int_add, +)
DEFINE_ARITH(int_sub, -)
DEFINE_ARITH(int_mul, *)
DEFINE_ARITH(int_div, /)
DEFINE_ARITH(int_mod, %)
DEFINE_ARITH(int_shl, <<)
DEFINE_ARITH(int_shr, >>)
DEFINE_ARITH(int_and, &)
DEFINE_ARITH(int_or,  |)
DEFINE_ARITH(int_xor, ^)


#define DEFINE_COMPARE(name, op) \
DEFINE_BUILTIN(name, \
	Word source2 = POP(); \
	Word source1 = POP(); \
	Word target = SPOT(0); \
\
	palint_t a;\
	palint_t b;\
	read_palint(&a, source1); \
	read_palint(&b, source2); \
\
	palbool_t ans = a op b; \
	*(palbool_t*)target = ans;\
	return code; \
)

DEFINE_COMPARE(int_eq, ==)
DEFINE_COMPARE(int_neq, !=)
DEFINE_COMPARE(int_smaller, <)
DEFINE_COMPARE(int_bigger, >)
DEFINE_COMPARE(int_le, <=)
DEFINE_COMPARE(int_ge, >=)


#define DEFINE_LOGIC(name, op) \
DEFINE_BUILTIN(name, \
	palbool_t* source = POP(); \
	palbool_t* target = SPOT(0); \
	*target = *source op *target; \
	return code; \
)

DEFINE_LOGIC(bool_and, &)
DEFINE_LOGIC(bool_or, |)
DEFINE_LOGIC(bool_xor, ^)

DEFINE_BUILTIN(bool_not,
    palbool_t* target = SPOT(0);
    *target = !(*target);
    return code;
)

void panic(VM* vm,long code){
	fflush(stdout);

	if(vm->panic_handler)
		longjmp(*vm->panic_handler,code);
	else{
		fprintf(stderr,"pal crashed without catching %ld\n",code);
		exit(code);
	}
}
