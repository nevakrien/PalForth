#ifndef CTYPES_H
#define CTYPES_H

#include "config.h"
#include "arena.h"
#include <setjmp.h>

typedef struct vm VM;
typedef struct code Code;

/**
 * type for both FFI and VM intrinsics
 * 
 * the return Code* is used for branching, for no branching return the original Code.
 * this weird convention was chosen because many calling conventions have the first argument be the first return
 */
typedef Code* (*XT)( Code*,VM*);
typedef void* Word;

typedef struct{
	Word* cur;
	intptr_t base; //avoid weird UB rules on comparing
	intptr_t end; //1 above last valid address
} Stack;
#define STACK_LIT(buffer,n) (Stack){buffer,(intptr_t)buffer,(intptr_t)(buffer+n)}

struct code {
	XT xt;
	Code* first_const;
	// Code* code_end;
	//data goes here
};

#define CODE_DATA(code) ((void *)((char *)(code) + sizeof(Code)))
#define CODE_FROM_DATA(data) ((void *)((char *)(data) - sizeof(Code)))


struct vm {
	Stack stack;
	
	Code* catch_point;
	Code* error_point;
	jmp_buf* panic_handler;

	Arena temp_mem;


#ifdef USE_ARENA
	Arena dict_mem; //can have memory shared to other VMs
#endif

};

#endif // CTYPES_H

