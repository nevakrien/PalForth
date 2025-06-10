#ifndef VM_H
#define VM_H

#include "config.h"
#include "arena.h"

#include "errors.h"
#include "ctypes.h"
#include "stack.h"
#include "string.h"


Code* excute_code(VM* vm,Code* code);

#define DEFINE_BUILTIN(name, body) \
	inline Code* name(Code* code, VM* vm) { \
		VM_LOG("executing " #name); \
		body \
	}
DEFINE_BUILTIN(inject,
	Word source = POP();
	Word target = POP();
	memmove(target, source, (intptr_t)code->code_start);
	return code;
)

DEFINE_BUILTIN(frame_alloc,
	STACK_ALLOC((intptr_t)code->code_start);
	return code;
)

DEFINE_BUILTIN(frame_free,
	STACK_FREE((intptr_t)code->code_start);
	return code;
)

DEFINE_BUILTIN(push_local,
	PUSH(&SPOT((intptr_t)code->code_start));
	return code;
)

DEFINE_BUILTIN(pick,
	PUSH(SPOT((intptr_t)code->code_start));
	return code;
)

DEFINE_BUILTIN(branch,
	if (*(palbool_t*)POP()) {
		return code + (intptr_t)code->code_start;
	}
	return code;
)

DEFINE_BUILTIN(jump,
	return code + (intptr_t)code->code_start;
)

DEFINE_BUILTIN(call_dyn,
	return *(Code**)POP();
)

DEFINE_BUILTIN(ret,
	return NULL;
)



#ifdef TEST
void test_vm();
#endif

#endif // VM_H

