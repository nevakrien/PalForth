#ifndef VM_H
#define VM_H

#include "config.h"

#ifdef TEST
void test_vm();
#endif

typedef void* Word;

typedef struct {
	intptr_t base; //avoid weird UB rules on comparing
	void** cur;
	intptr_t end;
} Stack;

#define STACK_LIT(buffer,n) (Stack){(intptr_t)buffer,buffer,(intptr_t)(buffer+n-1)}

typedef struct{
	const char* message;
} Exception;

struct vm;
typedef struct vm VM;
struct code;
typedef struct code Code;
typedef int (*XT)(VM*,Code*);

struct code {
	XT xt;
	//data goes here
};

#define CODE_DATA(code) ((void *)((char *)(code) + sizeof(Code)))


struct vm {
	Stack stack;
	Code* pc;
	Code* catch_point;

#ifdef USE_ARENA
	Arena dict_mem; //can have memory shared to other VMs
#endif

};

#endif // VM_H

